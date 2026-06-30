use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::glib;
use gtk::cairo;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct UsageBar {
        pub fraction: RefCell<f64>,
        pub label: RefCell<String>,
        pub sub_text: RefCell<String>,
        pub bar_color: RefCell<(f64, f64, f64)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UsageBar {
        const NAME: &'static str = "UsageBar";
        type Type = super::UsageBar;
        type ParentType = gtk::DrawingArea;
    }

    impl ObjectImpl for UsageBar {}
    impl WidgetImpl for UsageBar {}
    impl DrawingAreaImpl for UsageBar {}
}

glib::wrapper! {
    pub struct UsageBar(ObjectSubclass<imp::UsageBar>)
        @extends gtk::Widget, gtk::DrawingArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl UsageBar {
    pub fn new(label: &str, color: (f64, f64, f64)) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.set_content_height(40);
        obj.set_hexpand(true);

        *obj.imp().label.borrow_mut() = label.to_string();
        *obj.imp().bar_color.borrow_mut() = color;

        let weak = obj.downgrade();
        obj.set_draw_func(move |_, cr, w, h| {
            if let Some(this) = weak.upgrade() {
                Self::draw(&this, cr, w as f64, h as f64);
            }
        });

        obj
    }

    pub fn set_fraction(&self, fraction: f64, sub_text: &str) {
        *self.imp().fraction.borrow_mut() = fraction;
        *self.imp().sub_text.borrow_mut() = sub_text.to_string();
        self.queue_draw();
    }

    fn draw(this: &Self, cr: &cairo::Context, w: f64, h: f64) {
        let imp = this.imp();
        let frac = imp.fraction.borrow().clamp(0.0, 1.0);
        let label = imp.label.borrow();
        let sub = imp.sub_text.borrow();
        let (r, g, b) = *imp.bar_color.borrow();

        let bar_h = 22.0;
        let bar_y = (h - bar_h) / 2.0;
        let radius = 6.0;

        // Background track
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.15);
        rounded_rect(cr, 0.0, bar_y, w, bar_h, radius);
        cr.fill().ok();

        // Filled portion — skip if zero to avoid rendering artifacts
        let fill_w = w * frac;
        if fill_w > 0.5 {
            cr.set_source_rgb(r, g, b);
            rounded_rect(cr, 0.0, bar_y, fill_w, bar_h, radius);
            cr.fill().ok();
        }

        // Label text (left side, over the bar)
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(11.0);
        cr.move_to(8.0, h / 2.0 + 4.0);
        cr.show_text(&label).ok();

        // Sub text (right side)
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.7);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(11.0);
        let ext = cr.text_extents(&sub).unwrap();
        cr.move_to(w - ext.width() - 10.0, h / 2.0 + 4.0);
        cr.show_text(&sub).ok();
    }
}

fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::PI/2., 0.);
    cr.arc(x + w - r, y + h - r, r, 0., std::f64::consts::PI/2.);
    cr.arc(x + r, y + h - r, r, std::f64::consts::PI/2., std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.*std::f64::consts::PI/2.);
    cr.close_path();
}
