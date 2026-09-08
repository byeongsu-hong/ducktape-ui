//! Opt in to host mouse observations while constructing a subscription.
/// Call this inside the active branch of `App::subscription`, including when
/// reusing a previously constructed subscription. The recipe is unchanged.
pub fn observe<Message>(subscription: iced::Subscription<Message>) -> iced::Subscription<Message> {
    crate::slots::set_mouse_interest(true);
    subscription
}
