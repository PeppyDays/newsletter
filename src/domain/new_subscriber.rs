use uuid::Uuid;

use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;

pub struct NewSubscriber {
    pub id: Uuid,
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
