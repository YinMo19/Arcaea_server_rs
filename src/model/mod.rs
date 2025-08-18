pub mod character;
pub mod download;
pub mod notification;
pub mod present;
pub mod purchase;
pub mod score;
pub mod user;
pub mod world;

// Re-export commonly used types for convenience
pub use user::{
    AuthResponse, Login, LoginRequest, NewUser, User, UserAuth, UserCodeMapping, UserCredentials,
    UserExists, UserInfo, UserLoginDevice, UserLoginDto, UserRegisterDto,
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

pub use present::{
    CreatePresentItem, CreatePresentRequest, Present, PresentItem, PresentListResponse, UserPresent,
};

pub use purchase::{
    BundleItem, BundlePurchase, PackPurchaseRequest, PackSinglePurchaseResponse, Purchase,
    PurchaseItem, PurchaseList, RedeemRequest, RedeemResponse, SinglePurchaseRequest,
    SpecialItemPurchaseRequest, SpecialItemPurchaseResponse, StaminaPurchaseResponse,
};

pub use world::{
    MapEnterResponse, MapParser, Stamina, StepItem, StepReward, UserMap, UserWorldEntry,
    WorldAllResponse, WorldMap, WorldMapInfo, WorldMapResponse, WorldStep,
};
