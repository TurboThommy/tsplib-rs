use std::fmt;

/// DisplayDataType specifies how the display data is defined in the problem instance.
#[derive(Debug)]
pub enum DisplayDataType {
    // COORDS_DISPLAY, Display is generated from the node coordinates (default value if node coordinates are specified)
    CoordDisplay,

    // TWOD_DISPLAY, Explicit coordinates in 2D are given
    TwoDDisplay,

    // NO_DISPLAY, No graphical display is possible (default value if node coordinates are not specified)
    NoDisplay,
}

impl fmt::Display for DisplayDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DisplayDataType::CoordDisplay => "COORDS_DISPLAY",
            DisplayDataType::TwoDDisplay => "TWOD_DISPLAY",
            DisplayDataType::NoDisplay => "NO_DISPLAY",
        };
        write!(f, "{}", s)
    }
}
