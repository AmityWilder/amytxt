use raylib::prelude::*;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Selection {
    start: usize,
    len: Option<NonZeroUsize>,
}

impl Selection {
    pub const fn new() -> Self {
        Self {
            start: 0,
            len: None,
        }
    }
}

fn main() {
    let mut document = String::from("hello\nworld");
    let mut selection = Selection::default();
    let font_size = 20.0;
    let spacing = font_size / 10.0;
    let line_space = 2.0;
    let padding = Vector2::new(5.0, 5.0);
    let background_color = Color::new(27, 27, 27, 255);
    let foreground_color = Color::WHITE;
    let mut scroll: usize = 0;

    let (mut rl, thread) = init().title("Amitxt").resizable().build();
    let font = rl.get_font_default();

    let mut lines_per_screen =
        (rl.get_screen_height() as f32 - 2.0 * padding.y / (font_size + line_space)) as usize;

    while !rl.window_should_close() {
        if rl.is_window_resized() {
            lines_per_screen = (rl.get_screen_height() as f32
                - 2.0 * padding.y / (font_size + line_space))
                as usize;
        }

        scroll = scroll
            .saturating_sub_signed(rl.get_mouse_wheel_move() as isize)
            .min(document.lines().count());

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(background_color);
        for (screen_linenum, line) in document
            .lines()
            .skip(scroll)
            .take(lines_per_screen)
            .enumerate()
        {
            let doc_linenum = screen_linenum + scroll;
            d.draw_text_pro(
                &font,
                line,
                padding + Vector2::new(0.0, screen_linenum as f32 * (font_size + line_space)),
                Vector2::zero(),
                0.0,
                font_size,
                spacing,
                foreground_color,
            );
            // d.draw_rectangle_rec();
        }
    }
}
