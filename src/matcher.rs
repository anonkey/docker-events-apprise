use bollard_next::service::{
    EventActor, EventMessage, EventMessageScopeEnum, EventMessageTypeEnum,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn match_with_vector_f<DataType, MatcherType, Callback: FnOnce(&MatcherType, &DataType) -> bool>(
    matcher: Option<Vec<MatcherType>>,
    value: Option<DataType>,
    callback: Callback,
) -> bool
where
    Callback: Copy,
{
    match matcher {
        Some(values_vec) => match value {
            Some(value) => values_vec
                .iter()
                .any(|match_value| callback(match_value, &value)),
            None => false,
        },
        None => true,
    }
}

fn match_with_vector<DataType>(matcher: Option<Vec<DataType>>, value: Option<DataType>) -> bool
where
    DataType: std::cmp::PartialEq,
{
    match matcher {
        Some(values_vec) => match value {
            Some(value) => values_vec.iter().any(|match_value| *match_value == value),
            None => false,
        },
        None => true,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorMatcher {
    /// The ID of the object emitting the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Vec<String>>,

    /// Various key/value attributes of the object, depending on its type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<HashMap<String, Vec<String>>>>,
}

impl ActorMatcher {
    fn match_id(&self, actor: &EventActor) -> bool {
        match_with_vector(self.id.clone(), actor.id.clone())
    }

    fn match_attributes(&self, actor: &EventActor) -> bool {
        match_with_vector_f(
            self.attributes.clone(),
            actor.attributes.clone(),
            |matcher, value| {
                matcher.iter().all(|(key, match_values)| {
                    let actor_value = value.get(key);

                    match_with_vector(Some(match_values.clone()), actor_value.cloned())
                })
            },
        )
    }

    fn match_actor(&self, actor: &EventActor) -> bool {
        self.match_id(actor) && self.match_attributes(actor)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventMatcher {
    /// The type of object emitting the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<Vec<EventMessageTypeEnum>>,

    /// The type of event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<Vec<ActorMatcher>>,

    /// Scope of the event. Engine events are `local` scope. Cluster (Swarm) events are `swarm` scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<EventMessageScopeEnum>>,
}

impl EventMatcher {
    fn match_type(&self, event: &EventMessage) -> bool {
        match_with_vector(self.r#type.clone(), event.typ)
    }

    fn match_action(&self, event: &EventMessage) -> bool {
        match_with_vector_f(
            self.action.clone(),
            event.action.clone(),
            |to_compare, value| value.starts_with(to_compare),
        )
    }

    fn match_actor(&self, event: &EventMessage) -> bool {
        match_with_vector_f(self.actor.clone(), event.actor.clone(), |matcher, actor| {
            matcher.match_actor(actor)
        })
    }

    fn match_scope(&self, event: &EventMessage) -> bool {
        match_with_vector(self.scope.clone(), event.scope)
    }

    pub fn match_event(&self, event: &EventMessage) -> bool {
        self.match_action(event)
            && self.match_actor(event)
            && self.match_scope(event)
            && self.match_type(event)
    }
}
