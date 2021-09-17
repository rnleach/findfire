use strum::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, IntoStaticStr, EnumString, Display, EnumIter)]
pub enum Satellite {
    G16,
    G17,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, IntoStaticStr, EnumString, Display, EnumIter)]
pub enum Sector {
    #[strum(serialize = "FDCM")]
    Meso,
    #[strum(serialize = "FDCC")]
    Conus,
    #[strum(serialize = "FDCF")]
    FullDisk,
}
