use crate::{LinearRgba, Mix, SRgba};
use bevy::render::color::HslRepresentation;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

/// Color in Hue-Saturation-Lightness color space with alpha
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Hsla {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
    pub alpha: f32,
}

impl Hsla {
    /// Construct a new [`Hsla`] color from components.
    pub const fn new(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Self {
        Self {
            hue,
            saturation,
            lightness,
            alpha,
        }
    }

    /// Convert the Oklaba color to a tuple of components.
    #[inline]
    pub const fn to_components(&self) -> (f32, f32, f32, f32) {
        (self.hue, self.saturation, self.lightness, self.alpha)
    }

    /// Construct a new [`Oklaba`] color from components.
    #[inline]
    pub const fn from_components((l, a, b, alpha): (f32, f32, f32, f32)) -> Self {
        Self::new(l, a, b, alpha)
    }
}

impl Default for Hsla {
    fn default() -> Self {
        Self::new(0., 0., 0., 1.)
    }
}

impl Mix for Hsla {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            hue: self.hue * n_factor + other.hue * factor,
            saturation: self.saturation * n_factor + other.saturation * factor,
            lightness: self.lightness * n_factor + other.lightness * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl From<SRgba> for Hsla {
    fn from(value: SRgba) -> Self {
        let (h, s, l) =
            HslRepresentation::nonlinear_srgb_to_hsl([value.red, value.green, value.blue]);
        Self::new(h, s, l, value.alpha)
    }
}

impl From<LinearRgba> for Hsla {
    fn from(value: LinearRgba) -> Self {
        Hsla::from(SRgba::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRgba;

    macro_rules! assert_approx_eq {
        ($x:expr, $y:expr, $d:expr) => {
            if !($x - $y < $d || $y - $x < $d) {
                panic!();
            }
        };
    }

    #[test]
    fn test_to_from_srgba() {
        let hsla = Hsla::new(0.5, 0.5, 0.5, 1.0);
        let srgba: SRgba = hsla.into();
        let hsla2: Hsla = srgba.into();
        assert_approx_eq!(hsla.hue, hsla2.hue, 0.001);
        assert_approx_eq!(hsla.saturation, hsla2.saturation, 0.001);
        assert_approx_eq!(hsla.lightness, hsla2.lightness, 0.001);
        assert_approx_eq!(hsla.alpha, hsla2.alpha, 0.001);
    }

    #[test]
    fn test_to_from_linear() {
        let hsla = Hsla::new(0.5, 0.5, 0.5, 1.0);
        let linear: LinearRgba = hsla.into();
        let hsla2: Hsla = linear.into();
        assert_approx_eq!(hsla.hue, hsla2.hue, 0.001);
        assert_approx_eq!(hsla.saturation, hsla2.saturation, 0.001);
        assert_approx_eq!(hsla.lightness, hsla2.lightness, 0.001);
        assert_approx_eq!(hsla.alpha, hsla2.alpha, 0.001);
    }
}