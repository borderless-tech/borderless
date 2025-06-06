use sea_orm::entity::prelude::*;

pub type Package = Model;
pub type ActivePackage = ActiveModel;
pub type EntityPackage = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "packages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub app_name: Option<String>,
    pub app_module: Option<String>,
    pub pkg_type: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub source_id: i32,
    pub capabilities_id: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::source::Entity",
        from = "Column::SourceId",
        to = "super::source::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Sources,

    #[sea_orm(
        belongs_to = "super::capabilities::Entity",
        from = "Column::CapabilitiesId",
        to = "super::capabilities::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Capabilities,

    #[sea_orm(has_many = "super::package_author::Entity")]
    PackageAuthors,
}

impl Related<super::source::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sources.def()
    }
}

impl Related<super::capabilities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Capabilities.def()
    }
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        super::package_author::Relation::Authors.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::package_author::Relation::Packages.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
