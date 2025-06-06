use sea_orm::entity::prelude::*;

pub type GitInfo = Model;
pub type ActiveGitInfo = ActiveModel;
pub type EntityGitInfo = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "git_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
    pub repository: Option<String>,
    pub tag: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
