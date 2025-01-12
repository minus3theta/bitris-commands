use bitris::pieces::Shape;

use crate::internals::fuzzy_shape::FuzzyShape;
use crate::{ForEachVisitor, ShapeOrder};

/// Represents an order of shapes that includes fuzzy.
/// "Order" means affected by the hold operation.
/// Thus, it allows branches to be produced, indicating that they are not necessarily consumed from the head.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub(crate) struct FuzzyShapeOrder {
    shapes: Vec<FuzzyShape>,
}

impl FuzzyShapeOrder {
    #[inline]
    pub fn new(shapes: Vec<FuzzyShape>) -> Self {
        Self { shapes }
    }

    /// Expand unknown shapes to the order assumed as the shape of each.
    #[allow(dead_code)]
    fn expand_as_wildcard(&self) -> Vec<ShapeOrder> {
        struct Visitor {
            out: Vec<ShapeOrder>,
        }

        impl ForEachVisitor<[Shape]> for Visitor {
            fn visit(&mut self, shapes: &[Shape]) {
                self.out.push(ShapeOrder::new(shapes.to_vec()));
            }
        }

        let mut visitor = Visitor { out: Vec::new() };
        self.expand_as_wildcard_walk(&mut visitor);
        visitor.out
    }

    /// See `expand_as_wildcard()` for details.
    pub(crate) fn expand_as_wildcard_walk(&self, visitor: &mut impl ForEachVisitor<[Shape]>) {
        fn build(
            shapes: &Vec<FuzzyShape>,
            index: usize,
            buffer: &mut Vec<Shape>,
            visitor: &mut impl ForEachVisitor<[Shape]>,
        ) {
            if shapes.len() <= index {
                visitor.visit(buffer.as_slice());
                return;
            }

            match shapes[index] {
                FuzzyShape::Known(shape) => {
                    buffer[index] = shape;
                    build(shapes, index + 1, buffer, visitor);
                }
                FuzzyShape::Unknown => {
                    for shape in Shape::all_iter() {
                        buffer[index] = shape;
                        build(shapes, index + 1, buffer, visitor);
                    }
                }
            }
        }

        assert!(!self.shapes.is_empty());
        let mut buffer = Vec::<Shape>::with_capacity(self.shapes.len());
        buffer.resize(self.shapes.len(), Shape::T);
        build(&self.shapes, 0, &mut buffer, visitor);
    }
}

#[cfg(test)]
mod tests {

    use crate::internals::{FuzzyShape, FuzzyShapeOrder};
    use crate::ShapeOrder;

    #[test]
    fn fuzzy() {
        use super::Shape::*;
        use FuzzyShape::*;
        let fuzzy_shape_order = FuzzyShapeOrder::new(vec![Known(T), Unknown, Known(O)]);
        let orders = fuzzy_shape_order.expand_as_wildcard();
        assert_eq!(
            orders,
            vec![
                ShapeOrder::new(vec![T, T, O]),
                ShapeOrder::new(vec![T, I, O]),
                ShapeOrder::new(vec![T, O, O]),
                ShapeOrder::new(vec![T, L, O]),
                ShapeOrder::new(vec![T, J, O]),
                ShapeOrder::new(vec![T, S, O]),
                ShapeOrder::new(vec![T, Z, O]),
            ]
        );
    }
}
