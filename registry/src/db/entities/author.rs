use sea_orm::entity::prelude::*;

pub type Author = Model;
pub type ActiveAuthor = ActiveModel;
pub type EntityAuthor = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "authors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::package_author::Entity")]
    PackageAuthors,
}

impl Related<super::package::Entity> for Entity {
    fn to() -> RelationDef {
        super::package_author::Relation::Packages.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::package_author::Relation::Authors.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
