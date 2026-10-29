// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Scripted module boundary only. The original LibraryHolder algorithm,
// mutex/refcounts and actual boost::function remain compiled unchanged.
#pragma once
#include "fixture.hpp"
#include <filesystem>
#include <stdexcept>
#include <system_error>
#include <type_traits>
namespace boost::dll {
namespace fs { using error_code=std::error_code; }
namespace load_mode { inline constexpr int append_decorations=1,search_system_folders=2; }
class shared_library {
    bool loaded_=false;
public:
    bool is_loaded() const { return loaded_; }
    void load(const std::filesystem::path&,std::error_code& error,int) {
        using namespace MfsDelegateFixture;
        event("module.load");
        if(plan.flags&32) { throw std::runtime_error("scripted module load exception"); }
        if(plan.flags&1) { error=std::make_error_code(std::errc::no_such_file_or_directory);return; }
        loaded_=true;module_loaded=true;
    }
    bool has(const std::string& name) const {
        using namespace MfsDelegateFixture;
        if(name=="MaaWin32ControlUnitCreate") { event("symbol.create");return !(plan.flags&2); }
        if(name=="MaaWin32ControlUnitDestroy") { event("symbol.destroy");return !(plan.flags&4); }
        throw std::logic_error("unexpected source symbol lookup");
    }
    template<class F> F& get(const std::string&) const {
        if constexpr(std::is_same_v<F,decltype(::MaaWin32ControlUnitCreate)>) { return MfsDelegateFixture::create; }
        else { static_assert(std::is_same_v<F,decltype(::MaaWin32ControlUnitDestroy)>);return MfsDelegateFixture::destroy; }
    }
    void unload() {
        if(!loaded_) { throw std::logic_error("unexpected duplicate module unload"); }
        MfsDelegateFixture::event("module.unload");
        loaded_=false;MfsDelegateFixture::module_loaded=false;
    }
};
}
