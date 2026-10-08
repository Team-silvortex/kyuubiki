use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, Visitor, value::MapAccessDeserializer},
};
use std::collections::HashSet;
use std::{fmt, marker::PhantomData};

struct ObjectEntity<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ObjectEntity<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = ObjectEntity<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a graph entity object")
            }

            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(ObjectEntity)
            }
        }
        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

pub(crate) trait EntityWithId {
    fn id(&self) -> &str;
    fn id_mut(&mut self) -> &mut String;
}

pub(crate) fn deserialize_nodes<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + EntityWithId,
{
    deserialize_entities(deserializer, "n", "nodes")
}

pub(crate) fn deserialize_elements<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + EntityWithId,
{
    deserialize_entities(deserializer, "e", "elements")
}

fn deserialize_entities<'de, D, T>(
    deserializer: D,
    prefix: &str,
    namespace: &str,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + EntityWithId,
{
    let mut entities: Vec<T> = Vec::<ObjectEntity<T>>::deserialize(deserializer)?
        .into_iter()
        .map(|entity| entity.0)
        .collect();
    for (index, entity) in entities.iter_mut().enumerate() {
        if entity.id().is_empty() {
            *entity.id_mut() = format!("{prefix}{index}");
        }
    }
    // Check after generation: an explicit n0/e0 must not alias an unnamed entity.
    let mut ids = HashSet::with_capacity(entities.len());
    for (index, entity) in entities.iter().enumerate() {
        if !ids.insert(entity.id()) {
            return Err(D::Error::custom(format!(
                "duplicate {namespace} id at index {index}"
            )));
        }
    }
    Ok(entities)
}

macro_rules! entity_with_id {
    ($($type:ty),+ $(,)?) => {
        $(impl EntityWithId for $type {
            fn id(&self) -> &str { &self.id }
            fn id_mut(&mut self) -> &mut String { &mut self.id }
        })+
    };
}

entity_with_id!(
    crate::ThermalBar1dNodeInput,
    crate::ThermalBar1dElementInput,
    crate::HeatBar1dNodeInput,
    crate::HeatBar1dElementInput,
    crate::TransientHeatBar1dElementInput,
    crate::ElectrostaticBar1dNodeInput,
    crate::ElectrostaticBar1dElementInput,
    crate::MagnetostaticBar1dNodeInput,
    crate::MagnetostaticBar1dElementInput,
    crate::AdvectionDiffusionBar1dNodeInput,
    crate::AdvectionDiffusionBar1dElementInput,
    crate::HeatPlaneNodeInput,
    crate::HeatPlaneTriangleElementInput,
    crate::HeatPlaneQuadElementInput,
    crate::ElectrostaticPlaneNodeInput,
    crate::ElectrostaticPlaneTriangleElementInput,
    crate::ElectrostaticPlaneQuadElementInput,
);
