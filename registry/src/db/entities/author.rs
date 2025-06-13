use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use crate::error::Error;

pub type ActiveAuthor = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "authors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::package_author::Entity")]
    PackageAuthors,
}

impl Related<super::package_author::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PackageAuthors.def()
    }
}

impl Related<super::package::Entity> for Entity {
    fn to() -> RelationDef {
        super::package_author::Relation::Meta.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::package_author::Relation::Authors.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveAuthor {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        author: borderless_pkg::Author,
    ) -> Result<u64, Error> {
        let author = ActiveAuthor {
            id: NotSet,
            name: Set(author.name),
            email: Set(author.email),
        };

        let author_result = ActiveAuthor::insert(author, txn).await?;
        Ok(author_result.id)
    }
}
