#![cfg(feature = "bastion-observer")]
use azul::{prelude::*, widgets::{label::Label, button::Button}};

struct DataModel {
    counter: usize,
}

impl Layout for DataModel {
    fn layout(&self, _info: LayoutInfo<Self>) -> Dom<Self> {
        let label = Label::new(format!("{}", self.counter)).dom();
        let button = Button::with_label("Update counter").dom()
            .with_callback(On::MouseUp, Callback(update_counter));

        Dom::div()
            .with_child(label)
            .with_child(button)
    }
}

fn update_counter(event: CallbackInfo<DataModel>) -> UpdateScreen {
    event.state.data.counter += 1;
    Redraw
}
