use strum::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, IntoStaticStr, EnumString, Display, EnumIter)]
pub enum Satellite {
    G16,
    G17,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, IntoStaticStr, EnumString, Display, EnumIter)]
pub enum Sector {
    Meso,
    Conus,
    FullDisk,
}
