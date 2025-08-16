pub mod character;
pub mod user;

// Re-export commonly used types for convenience
pub use user::{
    Login, NewUser, User, UserAuth, UserCodeMapping, UserCredentials, UserExists, UserInfo,
    UserLoginDevice, UserLoginDto, UserRegisterDto,
};

pub use character::{
    Character, CharacterInfo, CharacterItem, NewUserCharacter, UserCharacter, UserCharacterFull,
};
