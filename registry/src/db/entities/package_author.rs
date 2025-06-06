use sea_orm::entity::prelude::*;

pub type PackageAuthors = Model;
pub type ActivePackageAuthors = ActiveModel;
pub type EntityPackageEntity = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "package_authors")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub package_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub author_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::package::Entity",
        from = "Column::PackageId",
        to = "super::package::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Packages,

    #[sea_orm(
        belongs_to = "super::author::Entity",
        from = "Column::AuthorId",
        to = "super::author::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Authors,
}

impl Related<super::package::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Packages.def()
    }
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Authors.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
