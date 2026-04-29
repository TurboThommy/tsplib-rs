use std::fmt;

/// EdgeWeightType specifies how the edge weights are defined in the problem instance.
#[derive(Debug)]
pub enum EdgeWeightType {
    // EXPLICIT, Weights are listed explicitly in the corresponding section
    Explicit,

    // EUC_2D, Weights are Euclidean distances in 2D
    Euc2D,

    // EUC_3D, Weights are Euclidean distances in 3D
    Euc3D,

    // MAX_2D, Weights are maximum distances in 2D
    Max2D,

    // MAX_3D, Weights are maximum distances in 3D
    Max3D,

    // MAN_2D, Weights are Manhattan distances in 2D
    Man2D,

    // MAN_3D, Weights are Manhattan distances in 3D
    Man3D,

    // CEIL_2D, Weights are Euclidean distances in 2D rounded up
    Ceil2D,

    // GEO, Weights are geographical distances
    Geo,

    // ATT, Special distance function for problems att48 and att532
    Att,

    // XRAY1, Special distance function for crystallography problems (Version 1)
    Xray1,

    // XRAY2, Special distance function for crystallography problems (Version 2)
    Xray2,

    // SPECIAL, There is a special distance function documented elsewhere
    Special,
}

impl fmt::Display for EdgeWeightType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeWeightType::Explicit => "EXPLICIT",
            EdgeWeightType::Euc2D => "EUC_2D",
            EdgeWeightType::Euc3D => "EUC_3D",
            EdgeWeightType::Max2D => "MAX_2D",
            EdgeWeightType::Max3D => "MAX_3D",
            EdgeWeightType::Man2D => "MAN_2D",
            EdgeWeightType::Man3D => "MAN_3D",
            EdgeWeightType::Ceil2D => "CEIL_2D",
            EdgeWeightType::Geo => "GEO",
            EdgeWeightType::Att => "ATT",
            EdgeWeightType::Xray1 => "XRAY1",
            EdgeWeightType::Xray2 => "XRAY2",
            EdgeWeightType::Special => "SPECIAL",
        };
        write!(f, "{}", s)
    }
}
