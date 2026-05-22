use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ab_glyph::FontArc;
use chrono::Local;
use image::codecs::png::PngEncoder;
use image::imageops::{overlay, resize, FilterType};
use image::{ColorType, ImageEncoder, Rgba, RgbaImage};
use imageproc::drawing::{draw_text_mut, text_size};

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
}

pub fn parse_score_image_mode(slug: &str) -> Option<ScoreImageMode> {
    match slug {
        "b30" => Some(ScoreImageMode::B30),
        "r10" => Some(ScoreImageMode::R10),
        "ap30" => Some(ScoreImageMode::Ap30),
        "sex30" => Some(ScoreImageMode::Sex30),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedScoreImage {
    pub mode: ScoreImageMode,
    pub entry_count: usize,
    pub url: String,
}

#[derive(Debug, Clone)]
struct ScoreImageProfile {
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
    rating: f64,
}

#[derive(Debug, Clone)]
struct ScoreImageChart {
    name: String,
    ratings: [i32; 5],
}

#[derive(Clone)]
struct ScoreImageRenderer {
    assets: Arc<ScoreImageAssets>,
    manifest_dir: PathBuf,
    song_dir: PathBuf,
}

struct ScoreImageAssets {
    asset_root: PathBuf,
    fonts: ScoreImageFonts,
    images: ScoreImageAssetImages,
}

struct ScoreImageFonts {
    exo_regular: CalibratedFont,
    exo_semibold: CalibratedFont,
    geosans: CalibratedFont,
    yahei: CalibratedFont,
    noto_sc: CalibratedFont,
}

struct ScoreImageAssetImages {
    back: RgbaImage,
    title: RgbaImage,
    plate: RgbaImage,
    diff_normal: Vec<RgbaImage>,
    rating_images: Vec<RgbaImage>,
}

#[derive(Clone, Copy)]
enum FontFamily {
    ExoRegular,
    ExoSemiBold,
    Geosans,
    Yahei,
    NotoSc,
}

impl FontFamily {
    fn scale_multiplier(self) -> f32 {
        match self {
            Self::ExoRegular => 1.33,
            Self::ExoSemiBold => 1.34,
            Self::Geosans => 1.14,
            Self::Yahei => 1.33,
            Self::NotoSc => 1.48,
        }
    }

    fn y_offset(self, logical_scale: f32) -> i32 {
        match self {
            Self::ExoRegular | Self::ExoSemiBold => (logical_scale * 0.03).round() as i32,
            Self::Yahei => (logical_scale * 0.015).round() as i32,
            Self::Geosans | Self::NotoSc => 0,
        }
    }
}

#[derive(Clone)]
struct CalibratedFont {
    inner: FontArc,
    family: FontFamily,
}

impl CalibratedFont {
    fn new(inner: FontArc, family: FontFamily) -> Self {
        Self { inner, family }
    }

    fn scale(&self, logical_scale: f32) -> f32 {
        logical_scale * self.family.scale_multiplier()
    }

    fn y_offset(&self, logical_scale: f32) -> i32 {
        self.family.y_offset(logical_scale)
    }
}

pub async fn generate_score_images(
    pool: &DbPool,
    user_id: i32,
    modes: &[ScoreImageMode],
) -> ArcResult<Vec<GeneratedScoreImage>> {
    let mut images = Vec::with_capacity(modes.len());

    for mode in modes {
        let entries = load_entries(pool, user_id, *mode).await?;
        images.push(GeneratedScoreImage {
            mode: *mode,
            entry_count: entries.len(),
            url: format!(
                "/web/api/score-images/{}.png?user_id={user_id}",
                mode.slug()
            ),
        });
    }

    Ok(images)
}

pub async fn generate_score_image_png(
    pool: &DbPool,
    user_id: i32,
    mode: ScoreImageMode,
) -> ArcResult<Vec<u8>> {
    let profile = load_profile(pool, user_id).await?;
    let entries = load_entries(pool, user_id, mode).await?;
    let charts = load_chart_map(pool, &entries).await?;
    let renderer = ScoreImageRenderer::new()?;

    tokio::task::spawn_blocking(move || {
        let image = renderer.render_best_list_image(&profile, &entries, &charts, mode)?;
        encode_png(&image)
    })
    .await
    .map_err(|err| ArcError::input(format!("成绩图渲染任务失败: {err}")))?
    .map_err(ArcError::input)
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

        if let Some(row) = sqlx::query!("SELECT * FROM chart WHERE song_id = ?", entry.song_id)
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
        let asset_root = score_image_asset_root();
        Ok(Self {
            assets: Arc::new(ScoreImageAssets::load(asset_root)?),
            manifest_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            song_dir: PathBuf::from(CONFIG.song_file_folder_path.trim()),
        })
    }

    fn render_best_list_image(
        &self,
        profile: &ScoreImageProfile,
        entries: &[ScoreImageEntry],
        charts: &HashMap<String, ScoreImageChart>,
        mode: ScoreImageMode,
    ) -> Result<RgbaImage, String> {
        let mut canvas = RgbaImage::from_pixel(2400, 3800, rgba(255, 255, 255));
        let fonts = &self.assets.fonts;
        let images = &self.assets.images;

        overlay(&mut canvas, &images.back, 0, 0);
        overlay(&mut canvas, &images.title, 0, 50);
        draw_text_with_outline(
            &mut canvas,
            mode.title(),
            800,
            70,
            150.0,
            &fonts.geosans,
            rgba(255, 255, 255),
            Some((1.0, rgba(0, 0, 0))),
        );

        let character_id = i64::from(profile.character_id);
        let char_img = self.load_character_art(character_id, 750, 750);
        overlay(&mut canvas, &char_img, 1500, 100);

        let profile_icon = self.load_character_icon(character_id, 250, 250);
        overlay(&mut canvas, &profile_icon, 200, 275);

        let now_ptt = rating_ptt_to_float(profile.rating_ptt);
        let ptt_img = resize(
            &images.rating_images[ptt_badge_index(now_ptt)],
            160,
            160,
            FilterType::CatmullRom,
        );
        overlay(&mut canvas, &ptt_img, 345, 400);

        let ptt_text = format_ptt_text(profile.rating_ptt);
        let ptt_x = if ptt_text.len() == 4 {
            380
        } else if ptt_text.chars().nth(1) == Some('1') {
            375
        } else {
            370
        };
        draw_text_with_outline(
            &mut canvas,
            &ptt_text,
            ptt_x,
            440,
            50.0,
            &fonts.exo_semibold,
            rgba(255, 255, 255),
            Some((3.0, rgba(70, 70, 70))),
        );

        draw_text_with_outline(
            &mut canvas,
            &profile.name,
            600,
            300,
            120.0,
            &fonts.geosans,
            rgba(255, 255, 255),
            Some((1.0, rgba(0, 0, 0))),
        );
        draw_text_with_outline(
            &mut canvas,
            &format!("ID: {}", profile.user_code),
            600,
            450,
            70.0,
            &fonts.geosans,
            rgba(200, 200, 200),
            Some((1.0, rgba(0, 0, 0))),
        );

        let mut ptt_sum = 0.0;
        let mut ptt_recent_sum = 0.0;

        for (index, entry) in entries.iter().enumerate() {
            let row = index as i64 / 3;
            let col = index as i64 % 3;
            let base_x = col * 800;
            let base_y = 850 + row * 280;

            overlay(&mut canvas, &images.plate, base_x, base_y);

            draw_text(
                &mut canvas,
                &format_score(entry.score),
                base_x as i32 + 300,
                base_y as i32 + 74,
                65.0,
                &fonts.exo_regular,
                rgba(10, 10, 10),
            );

            let cover = self.load_cover(&entry.song_id, 200, 200);
            overlay(&mut canvas, &cover, base_x + 70, base_y + 40);

            let chart = charts.get(&entry.song_id);
            let title = chart
                .map(|chart| chart.name.as_str())
                .unwrap_or(entry.song_id.as_str());
            draw_mixed_text(
                &mut canvas,
                title,
                base_x as i32 + 300,
                base_y as i32 + 40,
                &fonts.exo_regular,
                35.0,
                &fonts.yahei,
                35.0,
                rgba(10, 10, 10),
            );

            draw_right_aligned_text(
                &mut canvas,
                &format!("#{}", index + 1),
                35.0,
                &fonts.exo_regular,
                base_y as i32 + 35,
                2400 - col as i32 * 800 - 740,
                rgba(10, 10, 10),
                2400,
            );

            let difficulty = entry.difficulty.clamp(0, 4);
            overlay(
                &mut canvas,
                &images.diff_normal[difficulty as usize],
                base_x + 300,
                base_y + 157,
            );

            let const_tenths = chart
                .and_then(|chart| chart.ratings.get(difficulty as usize).copied())
                .filter(|rating| *rating >= 0)
                .unwrap_or_default();
            draw_text(
                &mut canvas,
                &format!(
                    "{} >> {}",
                    format_constant(const_tenths),
                    format_compact_float(entry.rating, 3)
                ),
                base_x as i32 + 410,
                base_y as i32 + 150,
                35.0,
                &fonts.exo_regular,
                rgba(10, 10, 10),
            );

            ptt_sum += entry.rating;
            if index < 10 {
                ptt_recent_sum += entry.rating;
            }

            draw_right_aligned_text(
                &mut canvas,
                &format!("{}d", days_since_timestamp(entry.time_played)),
                35.0,
                &fonts.exo_regular,
                base_y as i32 + 205,
                2400 - col as i32 * 800 - 740,
                rgba(100, 100, 100),
                2400,
            );

            draw_text_with_outline(
                &mut canvas,
                &format!(
                    "P: {} (+{})",
                    entry.perfect_count, entry.shiny_perfect_count
                ),
                base_x as i32 + 300,
                base_y as i32 + 195,
                30.0,
                &fonts.exo_regular,
                rgba(114, 73, 118),
                Some((1.0, rgba(114, 73, 118))),
            );
            draw_text_with_outline(
                &mut canvas,
                &format!("F: {}", entry.near_count),
                base_x as i32 + 525,
                base_y as i32 + 195,
                30.0,
                &fonts.exo_regular,
                rgba(172, 149, 58),
                Some((1.0, rgba(172, 149, 58))),
            );
            draw_text_with_outline(
                &mut canvas,
                &format!("L: {}", entry.miss_count),
                base_x as i32 + 600,
                base_y as i32 + 195,
                30.0,
                &fonts.exo_regular,
                rgba(120, 120, 120),
                Some((1.0, rgba(120, 120, 120))),
            );
        }

        match mode {
            ScoreImageMode::B30 => {
                let best30_avg = format_compact_float((ptt_sum / 30.0).round_to(2), 2);
                let recent10_avg =
                    format_compact_float(((now_ptt * 40.0 - ptt_sum) / 10.0).round_to(3), 3);
                let max_ptt =
                    format_compact_float(((ptt_sum + ptt_recent_sum) / 40.0).round_to(2), 2);
                draw_summary_block(&mut canvas, fonts, 200, "Best 30", &best30_avg);
                draw_summary_block(&mut canvas, fonts, 600, "Recent 10", &recent10_avg);
                draw_summary_block(&mut canvas, fonts, 1000, "Max PTT", &max_ptt);
            }
            ScoreImageMode::R10 => {
                let recent10_avg = format_compact_float((ptt_sum / 10.0).round_to(2), 2);
                draw_summary_block(&mut canvas, fonts, 200, "Recent 10", &recent10_avg);
            }
            ScoreImageMode::Ap30 | ScoreImageMode::Sex30 => {}
        }

        let generated_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        draw_text_with_outline(
            &mut canvas,
            &format!("Generated by Arcaea Server Web / Reyar @{generated_at}"),
            600,
            3720,
            50.0,
            &fonts.noto_sc,
            rgba(230, 230, 230),
            Some((1.0, rgba(100, 100, 255))),
        );

        Ok(canvas)
    }

    fn load_character_art(&self, character_id: i64, width: u32, height: u32) -> RgbaImage {
        self.load_resized_with_fallback(
            &self
                .assets
                .asset_root
                .join("char/1080")
                .join(format!("{character_id}.png")),
            &self.assets.asset_root.join("char/1080/0.png"),
            width,
            height,
            transparent(),
        )
    }

    fn load_character_icon(&self, character_id: i64, width: u32, height: u32) -> RgbaImage {
        self.load_resized_with_fallback(
            &self
                .assets
                .asset_root
                .join("char")
                .join(format!("{character_id}_icon.png")),
            &self.assets.asset_root.join("char/0_icon.png"),
            width,
            height,
            transparent(),
        )
    }

    fn load_cover(&self, song_id: &str, width: u32, height: u32) -> RgbaImage {
        let candidates = vec![
            self.assets
                .asset_root
                .join("covers")
                .join(format!("{song_id}.jpg")),
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
                return resize(&image.to_rgba8(), width, height, FilterType::CatmullRom);
            }
        }

        RgbaImage::from_pixel(width, height, rgba(25, 25, 25))
    }

    fn load_resized_with_fallback(
        &self,
        path: &Path,
        fallback: &Path,
        width: u32,
        height: u32,
        fill: Rgba<u8>,
    ) -> RgbaImage {
        load_image_rgba(path)
            .or_else(|_| load_image_rgba(fallback))
            .map(|img| resize(&img, width, height, FilterType::CatmullRom))
            .unwrap_or_else(|_| RgbaImage::from_pixel(width, height, fill))
    }
}

impl ScoreImageAssets {
    fn load(asset_root: PathBuf) -> ArcResult<Self> {
        let fonts = ScoreImageFonts {
            exo_regular: CalibratedFont::new(
                load_font(&asset_root.join("Exo-Regular.ttf"))?,
                FontFamily::ExoRegular,
            ),
            exo_semibold: CalibratedFont::new(
                load_font(&asset_root.join("Exo-SemiBold.ttf"))?,
                FontFamily::ExoSemiBold,
            ),
            geosans: CalibratedFont::new(
                load_font(&asset_root.join("GeosansLight.ttf"))?,
                FontFamily::Geosans,
            ),
            yahei: CalibratedFont::new(
                load_font(&asset_root.join("yahei.ttf"))?,
                FontFamily::Yahei,
            ),
            noto_sc: CalibratedFont::new(
                load_font(&asset_root.join("NotoSansCJKsc-Regular.otf"))?,
                FontFamily::NotoSc,
            ),
        };

        let images = ScoreImageAssetImages {
            back: load_image_rgba(&asset_root.join("back.png")).map_err(ArcError::input)?,
            title: resize(
                &load_image_rgba(&asset_root.join("title.png")).map_err(ArcError::input)?,
                745,
                200,
                FilterType::CatmullRom,
            ),
            plate: load_image_rgba(&asset_root.join("plate.png")).map_err(ArcError::input)?,
            diff_normal: ["pst.png", "prs.png", "ftr.png", "byd.png", "etr.png"]
                .into_iter()
                .map(|name| {
                    load_image_rgba(&asset_root.join(name))
                        .map(|img| resize(&img, 100, 30, FilterType::CatmullRom))
                        .map_err(ArcError::input)
                })
                .collect::<ArcResult<Vec<_>>>()?,
            rating_images: (0..8)
                .map(|idx| {
                    load_image_rgba(&asset_root.join(format!("rating_{idx}.png")))
                        .map_err(ArcError::input)
                })
                .collect::<ArcResult<Vec<_>>>()?,
        };

        Ok(Self {
            asset_root,
            fonts,
            images,
        })
    }
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

fn draw_summary_block(
    canvas: &mut RgbaImage,
    fonts: &ScoreImageFonts,
    x: i32,
    label: &str,
    value: &str,
) {
    draw_text_with_outline(
        canvas,
        label,
        x,
        620,
        70.0,
        &fonts.exo_regular,
        rgba(230, 230, 230),
        Some((1.0, rgba(100, 100, 100))),
    );
    draw_text_with_outline(
        canvas,
        value,
        x + 60,
        705,
        70.0,
        &fonts.exo_regular,
        rgba(230, 230, 230),
        Some((1.0, rgba(100, 100, 100))),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_mixed_text(
    canvas: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    font_en: &CalibratedFont,
    scale_en: f32,
    font_other: &CalibratedFont,
    scale_other: f32,
    fill: Rgba<u8>,
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        let ch_text = ch.to_string();
        let (font, scale) = if ch.is_ascii() {
            (font_en, scale_en)
        } else {
            (font_other, scale_other)
        };
        draw_text(canvas, &ch_text, cursor_x, y, scale, font, fill);
        cursor_x += text_width(font, scale, &ch_text);
        if cursor_x.rem_euclid(800) > 600 {
            draw_text(canvas, "...", cursor_x, y, scale, font, fill);
            break;
        }
    }
}

fn draw_right_aligned_text(
    canvas: &mut RgbaImage,
    text: &str,
    scale: f32,
    font: &CalibratedFont,
    y: i32,
    right_margin: i32,
    fill: Rgba<u8>,
    canvas_width: i32,
) {
    let width = text_width(font, scale, text);
    let x = canvas_width - width - right_margin;
    draw_text(canvas, text, x, y, scale, font, fill);
}

fn draw_text(
    canvas: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    scale: f32,
    font: &CalibratedFont,
    fill: Rgba<u8>,
) {
    draw_text_mut(
        canvas,
        fill,
        x,
        y + font.y_offset(scale),
        font.scale(scale),
        &font.inner,
        text,
    );
}

fn draw_text_with_outline(
    canvas: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    scale: f32,
    font: &CalibratedFont,
    fill: Rgba<u8>,
    stroke: Option<(f32, Rgba<u8>)>,
) {
    if let Some((stroke_width, stroke_fill)) = stroke {
        let radius = stroke_radius(stroke_width);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if dx.abs() + dy.abs() > radius.max(1) {
                    continue;
                }
                draw_text(canvas, text, x + dx, y + dy, scale, font, stroke_fill);
            }
        }
    }
    draw_text(canvas, text, x, y, scale, font, fill);
}

fn stroke_radius(width: f32) -> i32 {
    if width <= 0.0 {
        0
    } else {
        width.round().max(1.0) as i32
    }
}

fn text_width(font: &CalibratedFont, scale: f32, text: &str) -> i32 {
    text_size(font.scale(scale), &font.inner, text).0 as i32
}

fn load_font(path: &Path) -> ArcResult<FontArc> {
    let bytes = fs::read(path)
        .map_err(|err| ArcError::input(format!("读取成绩图字体失败 {}: {err}", path.display())))?;
    FontArc::try_from_vec(bytes)
        .map_err(|_| ArcError::input(format!("解析成绩图字体失败: {}", path.display())))
}

fn load_image_rgba(path: &Path) -> Result<RgbaImage, String> {
    image::open(path)
        .map(|img| img.to_rgba8())
        .map_err(|err| format!("读取成绩图资源失败 {}: {err}", path.display()))
}

fn score_image_asset_root() -> PathBuf {
    if let Ok(path) = env::var("SCORE_IMAGE_ASSET_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        PathBuf::from("assets/renderer"),
        manifest_dir.join("assets/renderer"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .unwrap_or_else(|| manifest_dir.join("assets/renderer"))
}

fn rgba(r: u8, g: u8, b: u8) -> Rgba<u8> {
    Rgba([r, g, b, 255])
}

fn transparent() -> Rgba<u8> {
    Rgba([0, 0, 0, 0])
}

fn format_score(score: i32) -> String {
    format!("{score:08}")
}

fn format_constant(value: i32) -> String {
    format!("{:.1}", value as f64 / 10.0)
}

fn rating_ptt_to_float(rating_ptt: i32) -> f64 {
    rating_ptt.max(0) as f64 / 100.0
}

fn format_ptt_text(rating_ptt: i32) -> String {
    format!("{:.2}", rating_ptt_to_float(rating_ptt))
}

fn ptt_badge_index(ptt: f64) -> usize {
    if ptt < 3.5 {
        0
    } else if ptt < 7.0 {
        1
    } else if ptt < 10.0 {
        2
    } else if ptt < 11.0 {
        3
    } else if ptt < 12.0 {
        4
    } else if ptt < 12.5 {
        5
    } else if ptt < 13.0 {
        6
    } else {
        7
    }
}

fn days_since_timestamp(time_played: i64) -> i64 {
    let sec = if time_played > 10_000_000_000 {
        time_played / 1000
    } else {
        time_played
    };
    if sec <= 0 {
        return 0;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_else(|_| Local::now().timestamp());
    now.saturating_sub(sec) / 86_400
}

fn format_compact_float(value: f64, decimals: usize) -> String {
    let mut s = format!("{:.*}", decimals, value);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.push('0');
        }
    }
    s
}

trait RoundTo {
    fn round_to(self, decimals: usize) -> f64;
}

impl RoundTo for f64 {
    fn round_to(self, decimals: usize) -> f64 {
        let factor = 10_f64.powi(decimals as i32);
        (self * factor).round() / factor
    }
}
