//! Starts a game's Il2Cpp runtime (without Unity) and checks every wrapped Unity
//! method exists with the expected signature. Set `HAO_IL2CPP_GAME` to the game
//! folder. Skipped otherwise.

#![cfg(target_os = "linux")]

use hao_il2cpp_unity::{runtime::Il2Cpp, Color, LoadSceneMode, RenderMode, TextMeshPro};

#[test]
fn unity_methods_match_the_game() {
    let Some(game) = std::env::var_os("HAO_IL2CPP_GAME").map(std::path::PathBuf::from) else {
        eprintln!("skipping: set HAO_IL2CPP_GAME to an Il2Cpp game folder");
        return;
    };
    let data_dir = std::fs::read_dir(&game)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path().join("il2cpp_data")))
        .find(|p| p.join("Metadata/global-metadata.dat").exists())
        .unwrap();
    let il2cpp =
        Il2Cpp::load(hao_il2cpp_unity::runtime::find_game_assembly(&game).unwrap()).unwrap();
    il2cpp.init_standalone(&data_dir).unwrap();

    let has_tmp = il2cpp
        .class(
            TextMeshPro::ASSEMBLY,
            TextMeshPro::NAMESPACE,
            TextMeshPro::NAME,
        )
        .is_ok();
    let failures: Vec<_> = hao_il2cpp_unity::check_bindings()
        .into_iter()
        .filter(|(name, _)| has_tmp || !name.starts_with("TMPro."))
        .collect();
    assert!(failures.is_empty(), "{:#?}", failures);

    assert_eq!(std::mem::size_of::<Color>(), 16);
    assert_eq!(LoadSceneMode::ADDITIVE.0, 1);
    assert_eq!(RenderMode::SCREEN_SPACE_OVERLAY.0, 0);
    // Hooks SceneManager.Internal_SceneLoaded. Nothing loads scenes without Unity.
    hao_il2cpp_unity::on_scene_loaded(|_, _| {}).unwrap();
    hao_il2cpp_unity::on_scene_loaded(|_, _| {}).unwrap();

    // Hooks Time.frameCount. The target must exist with the expected signature;
    // nothing drives frames without Unity, so the callback won't fire here.
    let frame_count = il2cpp
        .class("UnityEngine.CoreModule", "UnityEngine", "Time")
        .unwrap()
        .method("get_frameCount", 0)
        .unwrap();
    frame_count.check_types::<(), i32>().unwrap();
    hao_il2cpp_unity::on_update(|| {}).unwrap();
    hao_il2cpp_unity::on_update(|| {}).unwrap();

    // The render hook (Camera.FireOnPostRender) and GL bindings resolve above via
    // check_bindings; installing the hook is harmless without a render loop.
    assert_eq!(std::mem::size_of::<hao_il2cpp_unity::Quaternion>(), 16);
    hao_il2cpp_unity::on_render(|_camera| {}).unwrap();
}
