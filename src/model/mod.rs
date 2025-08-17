pub mod character;
pub mod download;
pub mod notification;
pub mod score;
pub mod user;

// Re-export commonly used types for convenience
pub use user::{
    Login, NewUser, User, UserAuth, UserCodeMapping, UserCredentials, UserExists, UserInfo,
    UserLoginDevice, UserLoginDto, UserRegisterDto,
};

pub use character::{
    Character, CharacterInfo, CharacterItem, CharacterValue, CoreItem, Level, NewUserCharacter,
    Skill, UserCharacter, UserCharacterFull, UserCharacterInfo,
};

pub use download::{
    BestScore, Chart, CourseTokenRequest, CourseTokenResponse, DownloadAudio, DownloadFile,
    DownloadSong, DownloadToken, RankEntry, Recent30, ScoreResponse, ScoreSubmission,
    SongplayToken, WorldTokenRequest, WorldTokenResponse,
};

pub use score::{Potential, Recent30Tuple, Score, UserPlay, UserScore};

pub use notification::{
    NewNotification, Notification, NotificationResponse, RoomInviteNotification,
};
