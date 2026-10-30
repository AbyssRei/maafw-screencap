// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included inside the original namespace after full public declarations.
// These are the full original virtual slots, required by the real ABI owner.
// None of them is exercised as an input implementation by this capture fixture.
bool LinuxControlUnitMgr::start_app(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::stop_app(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::click(int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::swipe(int,int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_down(int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_move(int,int,int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::touch_up(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::click_key(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::input_text(const std::string&) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::key_down(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::key_up(int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::scroll(int,int) {throw std::logic_error("input excluded");}
bool LinuxControlUnitMgr::relative_move(int,int) {throw std::logic_error("input excluded");}
