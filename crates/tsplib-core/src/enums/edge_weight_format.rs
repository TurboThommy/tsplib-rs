//! This module defines the EdgeWeightFormat enum, which specifies how the edge weights are formatted in the problem instance.
use std::fmt;

/// EdgeWeightFormat specifies how the edge weights are formatted in the problem instance.
#[derive(Debug, Clone, Copy)]
pub enum EdgeWeightFormat {
    /// FUNCTION, Weights are given by a function (see EdgeWeightType)
    Function,

    /// FULL_MATRIX, Weights are given by a full matrix
    FullMatrix,

    /// UPPER_ROW, Upper triangular matrix (row-wise without diagonal entries)
    UpperRow,

    /// LOWER_ROW, Lower triangular matrix (row-wise without diagonal entries)
    LowerRow,

    /// UPPER_DIAG_ROW, Upper triangular matrix (row-wise including diagonal entries)
    UpperDiagRow,

    /// LOWER_DIAG_ROW, Lower triangular matrix (row-wise including diagonal entries)
    LowerDiagRow,

    /// UPPER_COL, Upper triangular matrix (column-wise without diagonal entries)
    UpperCol,

    /// LOWER_COL, Lower triangular matrix (column-wise without diagonal entries)
    LowerCol,

    /// UPPER_DIAG_COL, Upper triangular matrix (column-wise including diagonal entries)
    UpperDiagCol,

    /// LOWER_DIAG_COL, Lower triangular matrix (column-wise including diagonal entries)
    LowerDiagCol,
}

impl fmt::Display for EdgeWeightFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeWeightFormat::Function => "FUNCTION",
            EdgeWeightFormat::FullMatrix => "FULL_MATRIX",
            EdgeWeightFormat::UpperRow => "UPPER_ROW",
            EdgeWeightFormat::LowerRow => "LOWER_ROW",
            EdgeWeightFormat::UpperDiagRow => "UPPER_DIAG_ROW",
            EdgeWeightFormat::LowerDiagRow => "LOWER_DIAG_ROW",
            EdgeWeightFormat::UpperCol => "UPPER_COL",
            EdgeWeightFormat::LowerCol => "LOWER_COL",
            EdgeWeightFormat::UpperDiagCol => "UPPER_DIAG_COL",
            EdgeWeightFormat::LowerDiagCol => "LOWER_DIAG_COL",
        };
        write!(f, "{}", s)
    }
}
