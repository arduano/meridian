#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MIDIColor(pub(crate) u32);

impl MIDIColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self((r as u32) << 16 | (g as u32) << 8 | b as u32)
    }

    pub fn new_from_hue(hue: f64) -> Self {
        let hue = hue.rem_euclid(360.0) / 60.0;
        let c = 0.78;
        let x = c * (1.0 - ((hue % 2.0) - 1.0).abs());
        let (r, g, b) = match hue as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = 0.12;
        Self::new(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }

    pub fn to_rgba(self, alpha: f32) -> [f32; 4] {
        [
            self.red() as f32 / 255.0,
            self.green() as f32 / 255.0,
            self.blue() as f32 / 255.0,
            alpha,
        ]
    }

    pub fn to_rgba_packed(self, alpha: u8) -> u32 {
        (self.red() as u32)
            | ((self.green() as u32) << 8)
            | ((self.blue() as u32) << 16)
            | ((alpha as u32) << 24)
    }

    pub fn red(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn green(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn blue(&self) -> u8 {
        self.0 as u8
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MIDIColorPair {
    pub left: MIDIColor,
    pub right: MIDIColor,
}

impl MIDIColorPair {
    pub fn new(left: MIDIColor, right: MIDIColor) -> Self {
        Self { left, right }
    }

    pub fn solid(color: MIDIColor) -> Self {
        Self {
            left: color,
            right: color,
        }
    }

    pub fn new_vec(tracks: usize) -> Vec<Self> {
        let count = tracks.max(1) * 16;
        let mut vec = Vec::with_capacity(count);
        for index in 0..count {
            let track = index / 16;
            let channel = index % 16;
            let color = MIDIColor::new_from_hue(((track + channel) as f64 * -16.0) % 360.0);
            vec.push(Self::solid(color));
        }
        vec
    }

    pub fn average(self) -> MIDIColor {
        MIDIColor::new(
            ((self.left.red() as u16 + self.right.red() as u16) / 2) as u8,
            ((self.left.green() as u16 + self.right.green() as u16) / 2) as u8,
            ((self.left.blue() as u16 + self.right.blue() as u16) / 2) as u8,
        )
    }
}
