use glam::*;
use kiwi_app::AppBuilder;
use kiwi_core::{
    asset_cache, camera::{active_camera, far}, main_scene, transform::*
};
use kiwi_ecs::{EntityData, World};
use kiwi_element::ElementComponentExt;
use kiwi_model::{model_def, ModelDef};
use kiwi_model_import::{MaterialFilter, ModelImportPipeline, ModelImportTransform, ModelTransform};
use kiwi_primitives::{Cube, Quad};
use kiwi_renderer::{color, materials::pbr_material::PbrMaterialFromUrl};
use kiwi_std::{
    asset_cache::AsyncAssetKeyExt, asset_url::{AbsAssetUrl, AssetUrl, TypedAssetUrl}, math::SphericalCoords
};
use reqwest::Url;

async fn init(world: &mut World) {
    let assets = world.resource(asset_cache()).clone();

    Quad.el().set(scale(), Vec3::ONE * 20.).spawn_static(world);

    let asset_pipelines = vec![
        {
            let fir_base =
                "https://dims-content.fra1.digitaloceanspaces.com/assets/models/Unity/Dynamic%20Nature%20-%20Mountain%20Tree%20Pack/";

            ModelImportPipeline::new()
                .add_step(ModelImportTransform::MergeUnityMeshLods {
                    url: AbsAssetUrl::parse(format!("{fir_base}Fir_02_Small.FBX")).unwrap(),
                    lod_cutoffs: None,
                })
                .add_step(ModelImportTransform::OverrideMaterial {
                    filter: MaterialFilter::by_name("M_leaves_Fir"),
                    material: Box::new(PbrMaterialFromUrl {
                        base_color: Some(AssetUrl::parse(format!("{fir_base}Textures/T_Fir_leaves_BC_T.TGA")).unwrap()),
                        ..Default::default()
                    }),
                })
        },
        {
            let grass_base = "https://dims-content.fra1.digitaloceanspaces.com/assets/models/Quixel/Grass_vlkhcbxia_2K_3dplant_ms/";
            let grass_atlas = PbrMaterialFromUrl {
                base_color: Some(AssetUrl::parse(format!("{grass_base}Textures/Atlas/vlkhcbxia_2K_Albedo.jpg")).unwrap()),
                opacity: Some(AssetUrl::parse(format!("{grass_base}Textures/Atlas/vlkhcbxia_2K_Opacity.jpg")).unwrap()),
                double_sided: Some(true),
                ..Default::default()
            };
            let grass_billboard = PbrMaterialFromUrl {
                base_color: Some(AssetUrl::parse(format!("{grass_base}Textures/Billboard/Billboard_2K_Albedo.jpg")).unwrap()),
                opacity: Some(AssetUrl::parse(format!("{grass_base}Textures/Billboard/Billboard_2K_Opacity.jpg")).unwrap()),
                alpha_cutoff: Some(0.1),
                double_sided: Some(true),
                ..Default::default()
            };
            ModelImportPipeline::new()
                .add_step(ModelImportTransform::MergeMeshLods {
                    lods: vec![
                        ModelImportPipeline::model(AbsAssetUrl::parse(format!("{grass_base}Var11/Var11_LOD0.fbx")).unwrap()).add_step(
                            ModelImportTransform::OverrideMaterial { filter: MaterialFilter::All, material: Box::new(grass_atlas.clone()) },
                        ),
                        ModelImportPipeline::model(AbsAssetUrl::parse(format!("{grass_base}Var11/Var11_LOD1.fbx")).unwrap()).add_step(
                            ModelImportTransform::OverrideMaterial { filter: MaterialFilter::All, material: Box::new(grass_atlas.clone()) },
                        ),
                        ModelImportPipeline::model(AbsAssetUrl::parse(format!("{grass_base}Var11/Var11_LOD2.fbx")).unwrap()).add_step(
                            ModelImportTransform::OverrideMaterial { filter: MaterialFilter::All, material: Box::new(grass_atlas.clone()) },
                        ),
                        ModelImportPipeline::model(AbsAssetUrl::parse(format!("{grass_base}Var11/Var11_LOD3.fbx")).unwrap()).add_step(
                            ModelImportTransform::OverrideMaterial { filter: MaterialFilter::All, material: Box::new(grass_atlas) },
                        ),
                        ModelImportPipeline::model(AbsAssetUrl::parse(format!("{grass_base}Var11/Var11_LOD4.fbx")).unwrap()).add_step(
                            ModelImportTransform::OverrideMaterial { filter: MaterialFilter::All, material: Box::new(grass_billboard) },
                        ),
                    ],
                    lod_cutoffs: None,
                })
                .add_step(ModelImportTransform::Transform(ModelTransform::Scale { scale: 5. }))
        },
        ModelImportPipeline::model(
            AbsAssetUrl::parse("https://dims-content.fra1.digitaloceanspaces.com/assets/models/Misc/Soldier.glb").unwrap(),
        ),
        ModelImportPipeline::model_raw(
            AbsAssetUrl::parse("https://dims-content.fra1.digitaloceanspaces.com/assets/models/PolyHaven/Barrel_01_4k.glb").unwrap(),
        ),
        ModelImportPipeline::model(
            AbsAssetUrl::parse("https://dims-content.fra1.digitaloceanspaces.com/assets/models/PolyHaven/Barrel_01_4k.glb").unwrap(),
        ),
    ];
    let mut model_defs = Vec::new();
    for pipeline in asset_pipelines.iter() {
        let model_url = pipeline.produce_local_model_url(&assets).await.unwrap();
        model_defs.push(ModelDef(TypedAssetUrl::new(Url::from_file_path(model_url).unwrap())));
    }

    // "Regular" spawning
    for (i, model_def) in model_defs.iter().enumerate() {
        let xy = vec2(i as f32 * 3., 0.);
        Cube.el().set(translation(), xy.extend(-0.9)).set(color(), vec4(0.3, 0.3, 0.3, 1.)).spawn_static(world);
        let model = model_def.get(&assets).await.unwrap();
        let entity = model.spawn(world, &Default::default());
        world.add_component(entity, translation(), xy.extend(0.1)).unwrap();
    }

    // Attaching
    for (i, mod_def) in model_defs.iter().enumerate() {
        let xy = vec2(i as f32 * 3., 3.);
        Cube.el().set(translation(), xy.extend(-0.9)).set(color(), vec4(0.3, 0.3, 0.3, 1.)).spawn_static(world);
        EntityData::new().set(model_def(), mod_def.clone()).set(translation(), xy.extend(0.1)).spawn(world);
    }

    kiwi_cameras::spherical::new(vec3(0., 0., 0.), SphericalCoords::new(std::f32::consts::PI / 4., std::f32::consts::PI / 4., 5.))
        .set(active_camera(), 0.)
        .set(main_scene(), ())
        .set(far(), 2000.)
        .spawn(world);
}

fn main() {
    env_logger::init();
    AppBuilder::simple().run(|app, runtime| {
        runtime.block_on(async { init(&mut app.world).await });
    });
}
