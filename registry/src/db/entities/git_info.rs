use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use crate::error::Error;

pub type ActiveGitInfo = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "git_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub commit_hash_short: String,
    pub commits_past_tag: Option<u64>,
    pub tag: Option<String>,
    pub dirty: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveGitInfo {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        git: borderless_pkg::git_info::GitInfo,
    ) -> Result<u64, Error> {
        let git = ActiveGitInfo {
            id: NotSet,
            commit_hash_short: Set(git.commit_hash_short),
            commits_past_tag: Set(git.commits_past_tag),
            tag: Set(git.tag),
            dirty: Set(git.dirty),
        };

        let git_result = ActiveGitInfo::insert(git, txn).await?;
        Ok(git_result.id)
    }
}
