use strum::{EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, IntoStaticStr, EnumString)]
pub enum Satellite {
    G16,
    G17,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, IntoStaticStr, EnumString)]
pub enum Sector {
    Meso,
    Conus,
    FullDisk,
}
