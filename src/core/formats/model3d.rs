//! 3D models, and 3D programs' scenes, shown by the preview they carry.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "model",
    name: "3D models",
    description: "STL, 3MF, OBJ, glTF, PLY, FBX and other 3D files.",
    icon: "cube",
    kind: AssetKind::Model3d,
    exts: &[
        "glb", "gltf", "obj", "stl", "3mf", "ply", "fbx", "3ds", "dae", "ifc",
    ],
};

/// Apart from the models, so switching their entry off leaves these their pictures.
pub const SCENES: Family = Family {
    id: "scene",
    name: "3D scenes",
    description: "Blender and Cinema 4D files, shown by the preview they carry.",
    icon: "box",
    kind: AssetKind::Model3d,
    exts: &["blend", "c4d"],
};
