use std::rc::Rc;

use bitris::prelude::*;
use bitris::srs::SrsKickTable;

use crate::{ClippedBoard, Pattern, PatternElement, ShapeCounter, TryBind};
use crate::pc_possible::{PcPossibleBulkExecutor, PcPossibleExecutorCreationError};

/// The binder to hold and tie settings for `PcPossibleBulkExecutor`.
#[derive(Clone, PartialEq, PartialOrd, Hash, Debug)]
pub struct PcPossibleBulkExecutorBinder<T: RotationSystem> {
    pub move_rules: MoveRules<T>,
    pub clipped_board: ClippedBoard,
    pub pattern: Rc<Pattern>,
    pub allows_hold: bool,
}

impl PcPossibleBulkExecutorBinder<SrsKickTable> {
    /// Making the executor with SRS. See `PcPossibleBulkExecutorBinder::default()` for more details.
    pub fn srs(move_type: MoveType) -> Self {
        PcPossibleBulkExecutorBinder::default(MoveRules::srs(move_type))
    }
}

impl<T: RotationSystem> PcPossibleBulkExecutorBinder<T> {
    /// Making the executor with default.
    ///
    /// The default values are as follows:
    ///   + move rules: from argument
    ///   + board: Blank
    ///   + height: 4 lines
    ///   + pattern: Factorial of all shapes (like `*p7`)
    ///   + allows hold: yes
    pub fn default(move_rules: MoveRules<T>) -> Self {
        Self {
            move_rules,
            clipped_board: ClippedBoard::try_new(Board64::blank(), 4).unwrap(),
            pattern: Rc::from(Pattern::new(vec![
                PatternElement::Factorial(ShapeCounter::one_of_each()),
            ])),
            allows_hold: true,
        }
    }
}

impl<'a, T: RotationSystem> TryBind<'a, PcPossibleBulkExecutor<'a, T>> for PcPossibleBulkExecutorBinder<T> {
    type Error = PcPossibleExecutorCreationError;

    fn try_bind(&'a self) -> Result<PcPossibleBulkExecutor<'a, T>, Self::Error> {
        PcPossibleBulkExecutor::try_new(
            &self.move_rules,
            self.clipped_board,
            self.pattern.as_ref(),
            self.allows_hold,
        )
    }
}


#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::str::FromStr;

    use bitris::prelude::*;

    use crate::{ClippedBoard, Pattern, PatternElement, ShapeCounter, TryBind};
    use crate::pc_possible::PcPossibleBulkExecutorBinder;

    #[test]
    fn reuse() {
        use PatternElement::*;

        let mut binder = PcPossibleBulkExecutorBinder::srs(MoveType::Softdrop);
        let board = Board64::from_str("
            ####......
            ####......
            ####......
            ####......
        ").unwrap();
        binder.clipped_board = ClippedBoard::try_new(board, 4).unwrap();

        let executor = binder.try_bind().unwrap();
        let result = executor.execute();
        assert_eq!(result.count_succeed(), 5040);

        let mut binder = binder.clone();
        let board = Board64::from_str("
            ####....##
            ###.....##
            ##......##
            ###.....##
        ").unwrap();
        binder.clipped_board = ClippedBoard::try_new(board, 4).unwrap();
        binder.pattern = Rc::from(Pattern::new(vec![
            Permutation(ShapeCounter::one_of_each(), 6),
        ]));
        let executor = binder.try_bind().unwrap();
        let result = executor.execute();
        assert_eq!(result.count_succeed(), 4088);
    }
}
