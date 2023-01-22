use bitris::prelude::*;
use fxhash::FxHashSet;
use itertools::Itertools;

use crate::{ClippedBoard, ShapeCounter};
use crate::all_pcs::{IndexedPieces, IndexNode, Nodes, PredefinedPiece, PredefinedPieceToAggregate};

trait PcAggregationChecker {
    fn checks(&self, shapes: Vec<Shape>) -> bool {
        true
    }
}

pub(crate) struct Aggregator {
    clipped_board: ClippedBoard,
    indexed_pieces: IndexedPieces<PredefinedPieceToAggregate>,
    using_rows_each_y: Vec<Lines>,
    width: usize,
    nodes: Nodes,
    spawn_position: BlPosition,
}

impl Aggregator {
    pub(crate) fn new(
        clipped_board: ClippedBoard,
        indexed_pieces: IndexedPieces<PredefinedPiece>,
        width: usize,
        nodes: Nodes,
        spawn_position: BlPosition,
    ) -> Self {
        let indexed_pieces = IndexedPieces::<PredefinedPieceToAggregate>::from(&indexed_pieces);

        let using_rows_each_y = {
            let mut vec = Vec::<Lines>::new();

            let height = clipped_board.height() as usize;

            for mino_index in 0..indexed_pieces.len() {
                let mino = &indexed_pieces[mino_index];
                for iy in 0..height {
                    vec.push(if mino.deleted_rows.test_at(iy) {
                        mino.using_rows
                    } else {
                        Lines::blank()
                    })
                }
            }

            vec
        };

        Self { clipped_board, indexed_pieces, using_rows_each_y, nodes, width, spawn_position }
    }

    pub(crate) fn aggregate(&self) -> u64 {
        if self.nodes.indexes.is_empty() {
            return 0;
        }

        struct PcAggregationCheckerImpl;
        impl PcAggregationChecker for PcAggregationCheckerImpl {}

        let mut filled = [Lines::blank(); 100];
        let mut results = Vec::new();
        self.aggregate_(0, 0, &mut filled, &mut results, &PcAggregationCheckerImpl)
    }

    pub(crate) fn aggregate_with_shape_counters(&self, shape_counters: &Vec<ShapeCounter>) -> u64 {
        if self.nodes.indexes.is_empty() {
            return 0;
        }

        struct PcAggregationCheckerImpl<'a> {
            shape_counters: &'a Vec<ShapeCounter>,
        }

        impl PcAggregationChecker for PcAggregationCheckerImpl<'_> {
            fn checks(&self, shapes: Vec<Shape>) -> bool {
                let counter = ShapeCounter::from(shapes);
                self.shape_counters.iter().any(|it| {
                    it.contains_all(&counter)
                })
            }
        }

        let checker = PcAggregationCheckerImpl { shape_counters };

        let mut filled = [Lines::blank(); 100];
        let mut results = Vec::new();
        self.aggregate_(0, 0, &mut filled, &mut results, &checker)
    }

    fn aggregate_(
        &self,
        index_id: usize,
        depth: usize,
        filled: &mut [Lines; 100],
        results2: &mut Vec<usize>,
        checker: &impl PcAggregationChecker,
    ) -> u64 {
        match self.nodes.indexes[index_id] {
            IndexNode::ToItem(next_item_id, item_length) => {
                let mut success = 0u64;

                let height = self.clipped_board.height() as usize;
                let (next_item_id, item_length) = (next_item_id as usize, item_length as usize);
                for index in next_item_id..(next_item_id + item_length) {
                    let item = &self.nodes.items[index];
                    let mino_index_and_lx = item.predefine_index as usize;
                    let mino_index = mino_index_and_lx / self.width;
                    let predefine = &self.indexed_pieces[mino_index];

                    let s = predefine.ys.iter().fold(Lines::blank(), |prev, y| {
                        prev | filled[depth * height + *y]
                    });

                    // 注目しているミノを置く前の絶対に揃えられないラインが削除されていないといけないか
                    if !(s & predefine.deleted_rows).is_blank() {
                        // 使っている
                        continue;
                    }

                    results2.push(mino_index_and_lx);

                    let next_depth = depth + 1;

                    let ni = next_depth * height;
                    let ci = depth * height;
                    let hi = mino_index * height;

                    // 揃えられないラインを更新
                    // temp = [y]ラインにブロックがあると、使用できないライン一覧が記録されている
                    // ミノXの[y]がdeletedKeyに指定されていると、Xのブロックのあるラインは先に揃えられなくなる
                    for j in 0..height {
                        filled[ni + j] = filled[ci + j] | self.using_rows_each_y[hi + j];
                    }

                    success += self.aggregate_(item.next_index_id, next_depth, filled, results2, checker);

                    results2.pop();
                }

                success
            }
            IndexNode::ToNextIndex(next_index_id) => {
                self.aggregate_(next_index_id as usize, depth, filled, results2, checker)
            }
            IndexNode::Complete => {
                let shapes = results2.iter()
                    .map(|&it| {
                        let mino_index = it / self.width;
                        let pre = &self.indexed_pieces[mino_index];
                        pre.piece.shape
                    })
                    .collect_vec();
                let s = if checker.checks(shapes) {
                    let x = results2.iter()
                        .map(|&it| {
                            let (mino_index, lx) = (it / self.width, it % self.width);
                            let pre = &self.indexed_pieces[mino_index];
                            let offset = Offset::new(lx as i32, 0);
                            let board = pre.locations.iter()
                                .map(|location| { location + offset })
                                .fold(Board64::blank(), |mut merge, location| {
                                    merge.set_at(location);
                                    merge
                                });
                            (pre, lx, board)
                        })
                        .collect_vec();
                    self.ok(&x)
                } else {
                    false
                };
                if s { 1 } else { 0 }
            }
            IndexNode::Abort => {
                0
            }
        }
    }

    fn ok(&self, results: &Vec<(&PredefinedPieceToAggregate, usize, Board64)>) -> bool {
        let mut hash_set = FxHashSet::<u64>::default();
        hash_set.reserve((1u64 << results.len()) as usize);

        let move_rules = MoveRules::srs(AllowMove::Softdrop);

        self.ok2(results, self.clipped_board.board(), (1u64 << results.len()) - 1, &mut hash_set, &move_rules)
    }

    fn ok2(&self, results: &Vec<(&PredefinedPieceToAggregate, usize, Board64)>, board_: Board64, remaining: u64, visited: &mut FxHashSet<u64>, move_rules: &MoveRules<SrsKickTable>) -> bool {
        let mut board2 = board_.clone();
        let deleted_key = board2.clear_lines();

        let mut rest2 = remaining;
        while rest2 != 0 {
            let bit = rest2 & (-(rest2 as i64)) as u64;

            // let next_used = used | bit;
            let index = bit.trailing_zeros() as usize;
            let (mino, lx, mino_board) = &results[index];
            let mino = *mino;

            // ミノを置くためのラインがすべて削除されている
            if (mino.deleted_rows & deleted_key) == mino.deleted_rows {
                let original_by = mino.ys[0];
                let mask = Lines::filled_up_to(original_by);

                let deleted_lines = mask & deleted_key;

                let by = original_by as i32 - deleted_lines.count() as i32;
                let placement = mino.piece.with(bl(*lx as i32, by));

                // Spawn on top of the well to avoid getting stuck.
                let spawn = mino.piece.shape.with(Orientation::North).with(self.spawn_position);

                if move_rules.can_reach(placement, board2, spawn) {
                    let next_rest = remaining - bit;
                    if next_rest == 0 {
                        return true;
                    }

                    if !visited.insert(next_rest) {
                        rest2 -= bit;
                        continue;
                    }

                    let mut next_field = board_.clone();
                    next_field.merge(mino_board);

                    if self.ok2(results, next_field, next_rest, visited, move_rules) {
                        return true;
                    }
                }
            }

            rest2 -= bit;
        }

        return false;
    }
}
