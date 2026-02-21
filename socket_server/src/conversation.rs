use uuid::Uuid;

#[derive(Clone)]
pub struct Conversation {
    pub id: Uuid,
    pub participants: Vec<Uuid>,
}

impl Conversation {
    pub fn new(participants: Vec<Uuid>) -> Conversation {
        let id = Uuid::new_v4();
        Conversation { id, participants }
    }
}
