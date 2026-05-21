use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ab_glyph::FontArc;
use base64::{engine::general_purpose, Engine as _};
use chrono::{Local, TimeZone};
use image::codecs::png::PngEncoder;
use image::imageops::{overlay, resize, FilterType};
use image::{ColorType, ImageEncoder, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;

use crate::config::CONFIG;
use crate::error::{ArcError, ArcResult};
use crate::DbPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreImageMode {
    B30,
    R10,
    Ap30,
    Sex30,
}

impl ScoreImageMode {
    pub fn slug(self) -> &'static str {
        match self {
            Self::B30 => "b30",
            Self::R10 => "r10",
            Self::Ap30 => "ap30",
            Self::Sex30 => "sex30",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::B30 => "Player Bests",
            Self::R10 => "Player Recents",
            Self::Ap30 => "Player PMs",
            Self::Sex30 => "Player Sexs",
        }
    }

    fn summary_label(self) -> &'static str {
        match self {
            Self::B30 => "B30",
            Self::R10 => "R10",
            Self::Ap30 => "AP30",
            Self::Sex30 => "SEX30",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedScoreImage {
    pub mode: ScoreImageMode,
    pub entry_count: usize,
    pub data_url: String,
}

#[derive(Debug, Clone)]
struct ScoreImageProfile {
    user_id: i32,
    name: String,
    user_code: String,
    rating_ptt: i32,
    character_id: i32,
}

#[derive(Debug, Clone)]
struct ScoreImageEntry {
    song_id: String,
    difficulty: i32,
    score: i32,
    shiny_perfect_count: i32,
    perfect_count: i32,
    near_count: i32,
    miss_count: i32,
    time_played: i64,
    clear_type: i32,
    rating: f64,
}

#[derive(Debug, Clone)]
struct ScoreImageChart {
    name: String,
    ratings: [i32; 5],
}

#[derive(Clone)]
struct ScoreImageRenderer {
    fonts: Arc<ScoreImageFonts>,
    manifest_dir: PathBuf,
    song_dir: PathBuf,
}

struct ScoreImageFonts {
    regular: FontArc,
    cjk: FontArc,
}

pub async fn generate_score_images(
    pool: &DbPool,
    user_id: i32,
    modes: &[ScoreImageMode],
) -> ArcResult<Vec<GeneratedScoreImage>> {
    let profile = load_profile(pool, user_id).await?;
    let renderer = ScoreImageRenderer::new()?;
    let mut images = Vec::with_capacity(modes.len());

    for mode in modes {
        let entries = load_entries(pool, user_id, *mode).await?;
        let charts = load_chart_map(pool, &entries).await?;
        let renderer = renderer.clone();
        let profile = profile.clone();
        let mode = *mode;
        let entry_count = entries.len();
        let png = tokio::task::spawn_blocking(move || {
            renderer.render_best_list_png(&profile, &entries, &charts, mode)
        })
        .await
        .map_err(|err| ArcError::input(format!("成绩图渲染任务失败: {err}")))?
        .map_err(ArcError::input)?;

        images.push(GeneratedScoreImage {
            mode,
            entry_count,
            data_url: format!(
                "data:image/png;base64,{}",
                general_purpose::STANDARD.encode(png)
            ),
        });
    }

    Ok(images)
}

async fn load_profile(pool: &DbPool, user_id: i32) -> ArcResult<ScoreImageProfile> {
    let row = sqlx::query!(
        "SELECT user_id, name, user_code, rating_ptt, character_id
         FROM user
         WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询玩家信息失败: {err}")))?
    .ok_or_else(|| ArcError::no_data("玩家不存在", -2))?;

    Ok(ScoreImageProfile {
        user_id: row.user_id,
        name: row.name.unwrap_or_else(|| row.user_id.to_string()),
        user_code: row.user_code.unwrap_or_else(|| "-".to_string()),
        rating_ptt: row.rating_ptt.unwrap_or_default(),
        character_id: row.character_id.unwrap_or_default(),
    })
}

async fn load_entries(
    pool: &DbPool,
    user_id: i32,
    mode: ScoreImageMode,
) -> ArcResult<Vec<ScoreImageEntry>> {
    match mode {
        ScoreImageMode::B30 => {
            let rows = sqlx::query!(
                "SELECT
                    COALESCE(song_id, '') as `song_id!: String`,
                    COALESCE(difficulty, 0) as `difficulty!: i32`,
                    COALESCE(score, 0) as `score!: i32`,
                    COALESCE(shiny_perfect_count, 0) as `shiny_perfect_count!: i32`,
                    COALESCE(perfect_count, 0) as `perfect_count!: i32`,
                    COALESCE(near_count, 0) as `near_count!: i32`,
                    COALESCE(miss_count, 0) as `miss_count!: i32`,
                    COALESCE(time_played, 0) as `time_played!: i64`,
                    COALESCE(clear_type, 0) as `clear_type!: i32`,
                    COALESCE(rating, 0) as `rating!: f64`
                 FROM best_score
                 WHERE user_id = ?
                 ORDER BY rating DESC, score DESC
                 LIMIT 30",
                user_id
            )
            .fetch_all(pool)
            .await
            .map_err(|err| ArcError::input(format!("查询 B30 失败: {err}")))?;

            Ok(rows
                .into_iter()
                .map(|row| ScoreImageEntry {
                    song_id: row.song_id,
                    difficulty: row.difficulty,
                    score: row.score,
                    shiny_perfect_count: row.shiny_perfect_count,
                    perfect_count: row.perfect_count,
                    near_count: row.near_count,
                    miss_count: row.miss_count,
                    time_played: row.time_played,
                    clear_type: row.clear_type,
                    rating: row.rating,
                })
                .collect())
        }
        ScoreImageMode::R10 => {
            let rows = sqlx::query!(
                "WITH ranked_songs AS (
                    SELECT r.*,
                           ROW_NUMBER() OVER (
                               PARTITION BY r.song_id
                               ORDER BY r.rating DESC, r.score DESC
                           ) AS song_rank
                    FROM recent30 r
                    WHERE r.user_id = ? AND r.song_id != ''
                 )
                 SELECT
                    COALESCE(song_id, '') as `song_id!: String`,
                    COALESCE(difficulty, 0) as `difficulty!: i32`,
                    COALESCE(score, 0) as `score!: i32`,
                    COALESCE(shiny_perfect_count, 0) as `shiny_perfect_count!: i32`,
                    COALESCE(perfect_count, 0) as `perfect_count!: i32`,
                    COALESCE(near_count, 0) as `near_count!: i32`,
                    COALESCE(miss_count, 0) as `miss_count!: i32`,
                    COALESCE(time_played, 0) as `time_played!: i64`,
                    COALESCE(clear_type, 0) as `clear_type!: i32`,
                    COALESCE(rating, 0) as `rating!: f64`
                 FROM ranked_songs
                 WHERE song_rank = 1
                 ORDER BY rating DESC, score DESC
                 LIMIT 10",
                user_id
            )
            .fetch_all(pool)
            .await
            .map_err(|err| ArcError::input(format!("查询 R10 失败: {err}")))?;

            Ok(rows
                .into_iter()
                .map(|row| ScoreImageEntry {
                    song_id: row.song_id,
                    difficulty: row.difficulty,
                    score: row.score,
                    shiny_perfect_count: row.shiny_perfect_count,
                    perfect_count: row.perfect_count,
                    near_count: row.near_count,
                    miss_count: row.miss_count,
                    time_played: row.time_played,
                    clear_type: row.clear_type,
                    rating: row.rating,
                })
                .collect())
        }
        ScoreImageMode::Ap30 => load_ap30_entries(pool, user_id).await,
        ScoreImageMode::Sex30 => load_sex30_entries(pool, user_id).await,
    }
}

async fn load_ap30_entries(pool: &DbPool, user_id: i32) -> ArcResult<Vec<ScoreImageEntry>> {
    let rows = sqlx::query!(
        "WITH filtered_data AS (
            SELECT *
            FROM best_score
            WHERE near_count = 0 AND miss_count = 0 AND user_id = ?
         ),
         ranked_data AS (
            SELECT *,
                   ROW_NUMBER() OVER (
                       PARTITION BY song_id, difficulty
                       ORDER BY score DESC
                   ) AS score_rank
            FROM filtered_data
         )
         SELECT
            COALESCE(song_id, '') as `song_id!: String`,
            COALESCE(difficulty, 0) as `difficulty!: i32`,
            COALESCE(score, 0) as `score!: i32`,
            COALESCE(shiny_perfect_count, 0) as `shiny_perfect_count!: i32`,
            COALESCE(perfect_count, 0) as `perfect_count!: i32`,
            COALESCE(near_count, 0) as `near_count!: i32`,
            COALESCE(miss_count, 0) as `miss_count!: i32`,
            COALESCE(time_played, 0) as `time_played!: i64`,
            COALESCE(clear_type, 0) as `clear_type!: i32`,
            COALESCE(rating, 0) as `rating!: f64`
         FROM ranked_data
         WHERE score_rank = 1
         ORDER BY rating DESC, score DESC
         LIMIT 30",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询 AP30 失败: {err}")))?;

    Ok(rows
        .into_iter()
        .map(|row| ScoreImageEntry {
            song_id: row.song_id,
            difficulty: row.difficulty,
            score: row.score,
            shiny_perfect_count: row.shiny_perfect_count,
            perfect_count: row.perfect_count,
            near_count: row.near_count,
            miss_count: row.miss_count,
            time_played: row.time_played,
            clear_type: row.clear_type,
            rating: row.rating,
        })
        .collect())
}

async fn load_sex30_entries(pool: &DbPool, user_id: i32) -> ArcResult<Vec<ScoreImageEntry>> {
    let rows = sqlx::query!(
        "WITH filtered_data AS (
            SELECT *
            FROM best_score
            WHERE ((near_count = 0 AND miss_count = 1) OR (near_count = 1 AND miss_count = 0))
              AND user_id = ?
         ),
         ranked_data AS (
            SELECT *,
                   ROW_NUMBER() OVER (
                       PARTITION BY song_id, difficulty
                       ORDER BY score DESC
                   ) AS score_rank
            FROM filtered_data
         )
         SELECT
            COALESCE(song_id, '') as `song_id!: String`,
            COALESCE(difficulty, 0) as `difficulty!: i32`,
            COALESCE(score, 0) as `score!: i32`,
            COALESCE(shiny_perfect_count, 0) as `shiny_perfect_count!: i32`,
            COALESCE(perfect_count, 0) as `perfect_count!: i32`,
            COALESCE(near_count, 0) as `near_count!: i32`,
            COALESCE(miss_count, 0) as `miss_count!: i32`,
            COALESCE(time_played, 0) as `time_played!: i64`,
            COALESCE(clear_type, 0) as `clear_type!: i32`,
            COALESCE(rating, 0) as `rating!: f64`
         FROM ranked_data
         WHERE score_rank = 1
         ORDER BY rating DESC, score DESC
         LIMIT 30",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询 Sex30 失败: {err}")))?;

    Ok(rows
        .into_iter()
        .map(|row| ScoreImageEntry {
            song_id: row.song_id,
            difficulty: row.difficulty,
            score: row.score,
            shiny_perfect_count: row.shiny_perfect_count,
            perfect_count: row.perfect_count,
            near_count: row.near_count,
            miss_count: row.miss_count,
            time_played: row.time_played,
            clear_type: row.clear_type,
            rating: row.rating,
        })
        .collect())
}

async fn load_chart_map(
    pool: &DbPool,
    entries: &[ScoreImageEntry],
) -> ArcResult<HashMap<String, ScoreImageChart>> {
    let mut seen = HashSet::new();
    let mut charts = HashMap::new();

    for entry in entries {
        if !seen.insert(entry.song_id.clone()) {
            continue;
        }

        if let Some(row) = sqlx::query!(
            "SELECT song_id, name, rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr
             FROM chart
             WHERE song_id = ?",
            entry.song_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|err| ArcError::input(format!("查询曲目信息失败: {err}")))?
        {
            charts.insert(
                row.song_id,
                ScoreImageChart {
                    name: row.name.unwrap_or_else(|| entry.song_id.clone()),
                    ratings: [
                        row.rating_pst.unwrap_or(-1),
                        row.rating_prs.unwrap_or(-1),
                        row.rating_ftr.unwrap_or(-1),
                        row.rating_byn.unwrap_or(-1),
                        row.rating_etr.unwrap_or(-1),
                    ],
                },
            );
        }
    }

    Ok(charts)
}

impl ScoreImageRenderer {
    fn new() -> ArcResult<Self> {
        let regular = load_first_font(&font_candidates("SCORE_IMAGE_FONT"))?;
        let cjk = load_first_font(&font_candidates("SCORE_IMAGE_CJK_FONT"))
            .unwrap_or_else(|_| regular.clone());
        Ok(Self {
            fonts: Arc::new(ScoreImageFonts { regular, cjk }),
            manifest_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            song_dir: PathBuf::from(CONFIG.song_file_folder_path.trim()),
        })
    }

    fn render_best_list_png(
        &self,
        profile: &ScoreImageProfile,
        entries: &[ScoreImageEntry],
        charts: &HashMap<String, ScoreImageChart>,
        mode: ScoreImageMode,
    ) -> Result<Vec<u8>, String> {
        let mut canvas = RgbaImage::from_pixel(2400, 3800, rgba(247, 248, 252));
        draw_background(&mut canvas, mode);
        self.draw_header(&mut canvas, profile, mode);
        self.draw_summary(&mut canvas, profile, entries, mode);

        for (index, entry) in entries.iter().enumerate() {
            let row = index / 3;
            let col = index % 3;
            let base_x = 80 + col as i32 * 760;
            let base_y = 820 + row as i32 * 280;
            self.draw_entry_card(&mut canvas, base_x, base_y, index, entry, charts);
        }

        let generated_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        draw_text(
            &mut canvas,
            &self.fonts.regular,
            60.0,
            760,
            3725,
            rgba(150, 152, 170),
            &format!("Generated by Arcaea Server Web / {generated_at}"),
        );

        encode_png(&canvas)
    }

    fn draw_header(
        &self,
        canvas: &mut RgbaImage,
        profile: &ScoreImageProfile,
        mode: ScoreImageMode,
    ) {
        draw_text(
            canvas,
            &self.fonts.regular,
            132.0,
            160,
            105,
            rgba(255, 255, 255),
            mode.title(),
        );
        draw_text(
            canvas,
            &self.fonts.cjk,
            96.0,
            160,
            275,
            rgba(255, 255, 255),
            &profile.name,
        );
        draw_text(
            canvas,
            &self.fonts.regular,
            54.0,
            166,
            415,
            rgba(229, 234, 255),
            &format!("ID: {}   UID: {}", profile.user_code, profile.user_id),
        );

        let ptt = format_ptt(profile.rating_ptt);
        draw_filled_rect_mut(
            canvas,
            Rect::at(1770, 120).of_size(440, 210),
            rgba(255, 255, 255),
        );
        draw_hollow_rect_mut(
            canvas,
            Rect::at(1770, 120).of_size(440, 210),
            rgba(216, 222, 245),
        );
        draw_text(
            canvas,
            &self.fonts.regular,
            38.0,
            1810,
            152,
            rgba(96, 101, 126),
            "PTT",
        );
        draw_text(
            canvas,
            &self.fonts.regular,
            94.0,
            1810,
            200,
            rgba(45, 48, 72),
            &ptt,
        );
        draw_text(
            canvas,
            &self.fonts.regular,
            34.0,
            1810,
            302,
            rgba(135, 140, 166),
            &format!("Character {}", profile.character_id),
        );
    }

    fn draw_summary(
        &self,
        canvas: &mut RgbaImage,
        profile: &ScoreImageProfile,
        entries: &[ScoreImageEntry],
        mode: ScoreImageMode,
    ) {
        let sum = entries.iter().map(|entry| entry.rating).sum::<f64>();
        let avg = if entries.is_empty() {
            0.0
        } else {
            sum / entries.len() as f64
        };
        let now_ptt = profile.rating_ptt as f64 / 100.0;

        let blocks = match mode {
            ScoreImageMode::B30 => vec![
                ("Best 30", format!("{avg:.2}")),
                (
                    "Recent 10",
                    format!("{:.3}", ((now_ptt * 40.0 - sum) / 10.0).max(0.0)),
                ),
                ("Count", entries.len().to_string()),
            ],
            ScoreImageMode::R10 => vec![
                ("Recent 10", format!("{avg:.2}")),
                ("Sum", format!("{sum:.4}")),
                ("Count", entries.len().to_string()),
            ],
            ScoreImageMode::Ap30 | ScoreImageMode::Sex30 => vec![
                (mode.summary_label(), format!("{avg:.2}")),
                ("Sum", format!("{sum:.4}")),
                ("Count", entries.len().to_string()),
            ],
        };

        for (index, (label, value)) in blocks.into_iter().enumerate() {
            let x = 80 + index as i32 * 520;
            draw_filled_rect_mut(
                canvas,
                Rect::at(x, 560).of_size(460, 150),
                rgba(255, 255, 255),
            );
            draw_hollow_rect_mut(
                canvas,
                Rect::at(x, 560).of_size(460, 150),
                rgba(222, 226, 241),
            );
            draw_text(
                canvas,
                &self.fonts.regular,
                36.0,
                x + 34,
                585,
                rgba(111, 116, 143),
                label,
            );
            draw_text(
                canvas,
                &self.fonts.regular,
                70.0,
                x + 34,
                630,
                rgba(45, 48, 72),
                &value,
            );
        }
    }

    fn draw_entry_card(
        &self,
        canvas: &mut RgbaImage,
        x: i32,
        y: i32,
        index: usize,
        entry: &ScoreImageEntry,
        charts: &HashMap<String, ScoreImageChart>,
    ) {
        draw_filled_rect_mut(
            canvas,
            Rect::at(x, y).of_size(700, 245),
            rgba(255, 255, 255),
        );
        draw_hollow_rect_mut(
            canvas,
            Rect::at(x, y).of_size(700, 245),
            rgba(224, 228, 242),
        );

        let cover = self.load_cover(&entry.song_id);
        overlay(canvas, &cover, i64::from(x + 24), i64::from(y + 24));

        let chart = charts.get(&entry.song_id);
        let title = chart
            .map(|chart| chart.name.as_str())
            .unwrap_or(entry.song_id.as_str());
        let title = ellipsize(title, 22);
        draw_text(
            canvas,
            &self.fonts.cjk,
            34.0,
            x + 250,
            y + 30,
            rgba(36, 39, 59),
            &title,
        );

        draw_right_text(
            canvas,
            &self.fonts.regular,
            34.0,
            x + 650,
            y + 28,
            rgba(120, 126, 152),
            &format!("#{}", index + 1),
        );

        draw_text(
            canvas,
            &self.fonts.regular,
            62.0,
            x + 250,
            y + 76,
            rgba(25, 28, 45),
            &format_score(entry.score),
        );

        let difficulty = entry.difficulty.clamp(0, 4);
        let (diff_label, diff_color) = difficulty_label(difficulty);
        draw_filled_rect_mut(
            canvas,
            Rect::at(x + 250, y + 155).of_size(84, 40),
            diff_color,
        );
        draw_text(
            canvas,
            &self.fonts.regular,
            24.0,
            x + 266,
            y + 163,
            rgba(255, 255, 255),
            diff_label,
        );

        let constant = chart
            .and_then(|chart| chart.ratings.get(difficulty as usize).copied())
            .unwrap_or(-1);
        draw_text(
            canvas,
            &self.fonts.regular,
            30.0,
            x + 350,
            y + 158,
            rgba(64, 68, 92),
            &format!("{} >> {:.3}", format_constant(constant), entry.rating),
        );

        draw_text(
            canvas,
            &self.fonts.regular,
            28.0,
            x + 250,
            y + 204,
            rgba(114, 73, 118),
            &format!(
                "BP/LP/F/L {}/{}/{}/{}",
                entry.shiny_perfect_count,
                (entry.perfect_count - entry.shiny_perfect_count).max(0),
                entry.near_count,
                entry.miss_count
            ),
        );
        draw_right_text(
            canvas,
            &self.fonts.regular,
            26.0,
            x + 650,
            y + 205,
            clear_type_color(entry.clear_type),
            clear_type_label(entry.clear_type),
        );
        draw_right_text(
            canvas,
            &self.fonts.regular,
            25.0,
            x + 650,
            y + 158,
            rgba(140, 145, 170),
            &days_since(entry.time_played),
        );
    }

    fn load_cover(&self, song_id: &str) -> RgbaImage {
        let candidates = vec![
            self.song_dir.join(song_id).join("base_256.jpg"),
            self.song_dir.join(song_id).join("base.jpg"),
            self.song_dir
                .join("songs")
                .join(song_id)
                .join("base_256.jpg"),
            self.song_dir.join("songs").join(song_id).join("base.jpg"),
            self.manifest_dir
                .join("songs")
                .join(song_id)
                .join("base_256.jpg"),
            self.manifest_dir
                .join("songs")
                .join(song_id)
                .join("base.jpg"),
            self.manifest_dir
                .join("songs")
                .join("songs")
                .join(song_id)
                .join("base_256.jpg"),
            self.manifest_dir
                .join("songs")
                .join("songs")
                .join(song_id)
                .join("base.jpg"),
        ];

        for path in candidates {
            if let Ok(image) = image::open(path) {
                return resize(&image.to_rgba8(), 190, 190, FilterType::CatmullRom);
            }
        }

        fallback_cover(song_id)
    }
}

fn draw_background(canvas: &mut RgbaImage, mode: ScoreImageMode) {
    let accent = match mode {
        ScoreImageMode::B30 => rgba(69, 85, 178),
        ScoreImageMode::R10 => rgba(42, 142, 126),
        ScoreImageMode::Ap30 => rgba(159, 89, 170),
        ScoreImageMode::Sex30 => rgba(190, 108, 72),
    };
    draw_filled_rect_mut(canvas, Rect::at(0, 0).of_size(2400, 510), accent);
    draw_filled_rect_mut(
        canvas,
        Rect::at(0, 510).of_size(2400, 30),
        rgba(222, 226, 241),
    );
    for idx in 0..16 {
        let x = idx * 190;
        draw_filled_rect_mut(
            canvas,
            Rect::at(x - 60, 0).of_size(90, 510),
            rgba(255, 255, 255).map_alpha(12),
        );
    }
}

fn fallback_cover(song_id: &str) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(190, 190, rgba(82, 91, 132));
    let hash = song_id
        .bytes()
        .fold(0u8, |acc, byte| acc.wrapping_add(byte));
    let accent = rgba(140 + (hash % 80), 120 + (hash % 90), 190);
    draw_filled_rect_mut(&mut image, Rect::at(16, 16).of_size(158, 158), accent);
    draw_hollow_rect_mut(
        &mut image,
        Rect::at(16, 16).of_size(158, 158),
        rgba(240, 242, 250),
    );
    image
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|err| format!("编码 PNG 失败: {err}"))?;
    Ok(bytes)
}

fn draw_text(
    canvas: &mut RgbaImage,
    font: &FontArc,
    size: f32,
    x: i32,
    y: i32,
    color: Rgba<u8>,
    text: &str,
) {
    draw_text_mut(canvas, color, x, y, size, font, text);
}

fn draw_right_text(
    canvas: &mut RgbaImage,
    font: &FontArc,
    size: f32,
    right_x: i32,
    y: i32,
    color: Rgba<u8>,
    text: &str,
) {
    let (width, _) = text_size(size, font, text);
    draw_text(canvas, font, size, right_x - width as i32, y, color, text);
}

fn font_candidates(env_key: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = env::var(env_key) {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.extend([
        manifest_dir.join("assets/fonts/Exo-Regular.ttf"),
        manifest_dir.join("assets/fonts/NotoSansCJKsc-Regular.otf"),
        manifest_dir.join("assets/renderer/Exo-Regular.ttf"),
        manifest_dir.join("assets/renderer/NotoSansCJKsc-Regular.otf"),
        PathBuf::from("/System/Library/Fonts/PingFang.ttc"),
        PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf"),
        PathBuf::from("/Library/Fonts/Arial Unicode.ttf"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.otf"),
        PathBuf::from("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ]);

    candidates
}

fn load_first_font(candidates: &[PathBuf]) -> ArcResult<FontArc> {
    let mut errors = Vec::new();
    for path in candidates {
        match load_font(path) {
            Ok(font) => return Ok(font),
            Err(err) => errors.push(format!("{}: {err}", path.display())),
        }
    }
    Err(ArcError::input(format!(
        "无法加载成绩图字体，请设置 SCORE_IMAGE_FONT: {}",
        errors.join("; ")
    )))
}

fn load_font(path: &Path) -> Result<FontArc, String> {
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    FontArc::try_from_vec(bytes).map_err(|_| "字体解析失败".to_string())
}

fn rgba(r: u8, g: u8, b: u8) -> Rgba<u8> {
    Rgba([r, g, b, 255])
}

trait Alpha {
    fn map_alpha(self, alpha: u8) -> Self;
}

impl Alpha for Rgba<u8> {
    fn map_alpha(mut self, alpha: u8) -> Self {
        self.0[3] = alpha;
        self
    }
}

fn format_ptt(rating_ptt: i32) -> String {
    if rating_ptt < 0 {
        "--".to_string()
    } else {
        format!("{:.2}", rating_ptt as f64 / 100.0)
    }
}

fn format_score(score: i32) -> String {
    format!("{score:08}")
}

fn format_constant(value: i32) -> String {
    if value < 0 {
        "-".to_string()
    } else {
        format!("{:.1}", value as f64 / 10.0)
    }
}

fn difficulty_label(difficulty: i32) -> (&'static str, Rgba<u8>) {
    match difficulty {
        0 => ("PST", rgba(89, 142, 205)),
        1 => ("PRS", rgba(89, 174, 116)),
        2 => ("FTR", rgba(146, 87, 174)),
        3 => ("BYD", rgba(188, 74, 88)),
        4 => ("ETR", rgba(198, 137, 55)),
        _ => ("UNK", rgba(110, 110, 120)),
    }
}

fn clear_type_label(clear_type: i32) -> &'static str {
    match clear_type {
        3 => "PM",
        2 => "FR",
        5 => "HC",
        1 => "NC",
        4 => "EC",
        _ => "TL",
    }
}

fn clear_type_color(clear_type: i32) -> Rgba<u8> {
    match clear_type {
        3 => rgba(126, 91, 194),
        2 => rgba(58, 128, 198),
        5 => rgba(196, 92, 70),
        1 => rgba(75, 145, 95),
        4 => rgba(107, 154, 172),
        _ => rgba(125, 128, 145),
    }
}

fn days_since(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "-".to_string();
    }
    let seconds = if timestamp > 10_000_000_000 {
        timestamp / 1000
    } else {
        timestamp
    };
    let Some(played_at) = Local.timestamp_opt(seconds, 0).single() else {
        return "-".to_string();
    };
    let days = Local::now()
        .signed_duration_since(played_at)
        .num_days()
        .max(0);
    format!("{days}d")
}

fn ellipsize(value: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            result.push_str("...");
            return result;
        }
        result.push(ch);
    }
    result
}
