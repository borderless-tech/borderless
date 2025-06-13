use sea_orm::{
    entity::{prelude::*, ActiveValue::Set},
    DatabaseTransaction,
};

use crate::error::Error;

pub type ActivePackageAuthors = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "package_authors")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub meta_id: u64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub author_id: u64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::meta::Entity",
        from = "Column::MetaId",
        to = "super::meta::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Meta,

    #[sea_orm(
        belongs_to = "super::author::Entity",
        from = "Column::AuthorId",
        to = "super::author::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Authors,
}

impl Related<super::meta::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Meta.def()
    }
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Authors.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl ActivePackageAuthors {
    pub async fn from_ids(
        txn: &DatabaseTransaction,
        meta_id: u64,
        author_id: u64,
    ) -> Result<(), Error> {
        let meta_pkg = ActivePackageAuthors {
            meta_id: Set(meta_id),
            author_id: Set(author_id),
        };

        ActivePackageAuthors::insert(meta_pkg, txn).await?;
        Ok(())
    }
}
