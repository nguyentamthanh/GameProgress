use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_newtype!(StudioId);
id_newtype!(UserId);
id_newtype!(GameId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudioRole {
    Admin,
    Producer,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStage {
    Idea,
    Prototype,
    VerticalSlice,
    SoftLaunch,
    Live,
    Stopped,
}

impl GameStage {
    pub const ALL: [GameStage; 6] = [
        Self::Idea,
        Self::Prototype,
        Self::VerticalSlice,
        Self::SoftLaunch,
        Self::Live,
        Self::Stopped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idea => "idea",
            Self::Prototype => "prototype",
            Self::VerticalSlice => "vertical_slice",
            Self::SoftLaunch => "soft_launch",
            Self::Live => "live",
            Self::Stopped => "stopped",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityMeta {
    pub version: i64,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl EntityMeta {
    pub fn new(now: OffsetDateTime) -> Self {
        Self {
            version: 1,
            updated_at: now,
            deleted_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub id: GameId,
    pub studio_id: StudioId,
    pub name: String,
    pub genre: Option<String>,
    pub platforms: Vec<String>,
    pub stage: GameStage,
    pub owner_id: UserId,
    pub store_url: Option<String>,
    pub meta: EntityMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub user_id: UserId,
    pub studio_id: StudioId,
    pub studio_role: StudioRole,
    pub game_grants: Vec<GameId>,
}

pub fn can_view_game(actor: &Actor, game: &Game) -> bool {
    if actor.studio_id != game.studio_id || game.meta.deleted_at.is_some() {
        return false;
    }
    match actor.studio_role {
        StudioRole::Admin | StudioRole::Producer => true,
        StudioRole::Member | StudioRole::Viewer => {
            game.owner_id == actor.user_id || actor.game_grants.contains(&game.id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_game(studio_id: StudioId, owner_id: UserId, deleted: bool) -> Game {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut meta = EntityMeta::new(now);
        if deleted {
            meta.deleted_at = Some(now);
        }
        Game {
            id: GameId::new(),
            studio_id,
            name: "Sample".into(),
            genre: None,
            platforms: Vec::new(),
            stage: GameStage::Idea,
            owner_id,
            store_url: None,
            meta,
        }
    }

    fn sample_actor(
        user_id: UserId,
        studio_id: StudioId,
        studio_role: StudioRole,
        game_grants: Vec<GameId>,
    ) -> Actor {
        Actor {
            user_id,
            studio_id,
            studio_role,
            game_grants,
        }
    }

    #[test]
    fn admin_sees_game_in_their_studio() {
        let studio = StudioId::new();
        let owner = UserId::new();
        let game = sample_game(studio, owner, false);
        let actor = sample_actor(UserId::new(), studio, StudioRole::Admin, vec![]);
        assert!(can_view_game(&actor, &game));
    }

    #[test]
    fn admin_does_not_see_another_studios_game() {
        let game = sample_game(StudioId::new(), UserId::new(), false);
        let actor = sample_actor(UserId::new(), StudioId::new(), StudioRole::Admin, vec![]);
        assert!(!can_view_game(&actor, &game));
    }

    #[test]
    fn producer_sees_game_in_their_studio() {
        let studio = StudioId::new();
        let game = sample_game(studio, UserId::new(), false);
        let actor = sample_actor(UserId::new(), studio, StudioRole::Producer, vec![]);
        assert!(can_view_game(&actor, &game));
    }

    #[test]
    fn member_owner_sees_game() {
        let studio = StudioId::new();
        let owner = UserId::new();
        let game = sample_game(studio, owner, false);
        let actor = sample_actor(owner, studio, StudioRole::Member, vec![]);
        assert!(can_view_game(&actor, &game));
    }

    #[test]
    fn member_with_grant_sees_game() {
        let studio = StudioId::new();
        let game = sample_game(studio, UserId::new(), false);
        let actor = sample_actor(UserId::new(), studio, StudioRole::Member, vec![game.id]);
        assert!(can_view_game(&actor, &game));
    }

    #[test]
    fn member_without_grant_and_not_owner_does_not_see_game() {
        let studio = StudioId::new();
        let game = sample_game(studio, UserId::new(), false);
        let actor = sample_actor(UserId::new(), studio, StudioRole::Member, vec![]);
        assert!(!can_view_game(&actor, &game));
    }

    #[test]
    fn viewer_without_grant_does_not_see_game() {
        let studio = StudioId::new();
        let game = sample_game(studio, UserId::new(), false);
        let actor = sample_actor(UserId::new(), studio, StudioRole::Viewer, vec![]);
        assert!(!can_view_game(&actor, &game));
    }

    #[test]
    fn viewer_with_grant_sees_game() {
        let studio = StudioId::new();
        let game = sample_game(studio, UserId::new(), false);
        let actor = sample_actor(UserId::new(), studio, StudioRole::Viewer, vec![game.id]);
        assert!(can_view_game(&actor, &game));
    }

    #[test]
    fn nobody_sees_a_soft_deleted_game() {
        let studio = StudioId::new();
        let owner = UserId::new();
        let game = sample_game(studio, owner, true);
        for role in [
            StudioRole::Admin,
            StudioRole::Producer,
            StudioRole::Member,
            StudioRole::Viewer,
        ] {
            let actor = sample_actor(owner, studio, role, vec![game.id]);
            assert!(!can_view_game(&actor, &game), "{role:?}");
        }
    }

    #[test]
    fn stage_wire_values_roundtrip() {
        for stage in GameStage::ALL {
            let wire = stage.as_str();
            assert_eq!(GameStage::parse(wire), Some(stage));
            let json = serde_json::to_string(&stage).unwrap();
            assert_eq!(json, format!("\"{wire}\""));
            let parsed: GameStage = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, stage);
        }
        assert_eq!(GameStage::parse("Idea"), None);
        assert_eq!(GameStage::parse("shipped"), None);
    }

    #[test]
    fn new_ids_are_uuid_version_7() {
        for id in [
            StudioId::new().as_uuid(),
            UserId::new().as_uuid(),
            GameId::new().as_uuid(),
        ] {
            assert_eq!(id.get_version().map(|version| version as u8), Some(7));
        }
    }
}
