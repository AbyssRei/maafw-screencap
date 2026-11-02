// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Run the existing producer in a Rust process. Do not rewrite the source
// capture class, its callback/wait, or any old oracle/scene entry.
#include "same-binary-scene.h"
#define main mfs_mac_scene_fixture_main
#include "source-context.mm"
#undef main

uint32_t mfs_mac_same_binary_scene_version(void) noexcept { return 1; }
int32_t mfs_mac_same_binary_scene_run(uint32_t mode,const char* executable,const char* directory) noexcept {
    if(mode>1 || !executable || !directory) { return 64; }
    try {
        // Owned mutable argv storage, valid throughout the original main.
        std::string program(executable),path(directory);
        std::string operation=mode==0 ? "display-scene" : "display-scene-cg";
        char* arguments[]={program.data(),operation.data(),path.data(),nullptr};
        return mfs_mac_scene_fixture_main(3,arguments);
    }catch(...) { return 2; }
}
uint32_t mfs_mac_full_display_scene_version(void) noexcept {return 1;}
int32_t mfs_mac_full_display_scene_run(uint32_t mode,const char* executable,const char* directory) noexcept {
    if(mode>1 || !executable || !directory) {return 64;}
    try {
        std::string program(executable),path(directory),operation="display-scene-cg-full";
        char* arguments[]={program.data(),operation.data(),path.data(),nullptr};
        return mfs_mac_scene_fixture_main(3,arguments);
    }catch(...) {return 2;}
}
uint32_t mfs_mac_full_display_witness_version(void) noexcept {return 1;}
int32_t mfs_mac_full_display_witness_run(uint32_t mode,const char* executable,const char* directory) noexcept {
    if(mode>1 || !executable || !directory) {return 64;}
    try {
        std::string program(executable),path(directory),operation="display-scene-cg-full-witness";
        char* arguments[]={program.data(),operation.data(),path.data(),nullptr};
        return mfs_mac_scene_fixture_main(3,arguments);
    }catch(...) {return 2;}
}
