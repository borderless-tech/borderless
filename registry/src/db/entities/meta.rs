use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use crate::{
    db::entities::{author::ActiveAuthor, package_author::ActivePackageAuthors},
    error::Error,
};

pub type ActiveMeta = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "meta")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::package_author::Entity")]
    PackageAuthor,

    #[sea_orm(has_many = "super::meta::Entity")]
    Meta,
}

impl Related<super::package_author::Entity> for Entity {
    fn to() -> RelationDef {
        super::author::Relation::PackageAuthors.def()
    }
}

impl Related<super::meta::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Meta.def()
    }
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        super::package_author::Relation::Authors.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::package_author::Relation::Meta.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveMeta {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        meta: borderless_pkg::PkgMeta,
    ) -> Result<u64, Error> {
        let meta_model = ActiveMeta {
            id: NotSet,
            description: Set(meta.description),
            documentation: Set(meta.documentation),
            license: Set(meta.license),
            repository: Set(meta.repository),
        };

        let meta_model_result = ActiveMeta::insert(meta_model, txn).await?;
        for author in meta.authors {
            let author_id = ActiveAuthor::from_model(txn, author).await?;
            ActivePackageAuthors::from_ids(txn, meta_model_result.id, author_id).await?;
        }
        Ok(meta_model_result.id)
    }
}
