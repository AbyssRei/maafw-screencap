// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Screenshot-only class surface for the extracted exact function bodies.
// Include inside the chosen private namespace after defining ScreencapBase,
// ScreenCaptureKitScreencap and mfs_mac_input_attempt. Input=None is mandatory.
struct InputBase { virtual ~InputBase()=default; };
struct GlobalEventInput : InputBase {
    explicit GlobalEventInput(uint32_t) { mfs_mac_input_attempt();throw std::logic_error("input excluded"); }
};
struct PostToPidInput : InputBase {
    explicit PostToPidInput(uint32_t) { mfs_mac_input_attempt();throw std::logic_error("input excluded"); }
};
struct MacOSControlUnitMgr {
    MacOSControlUnitMgr(uint32_t,MaaMacOSScreencapMethod,MaaMacOSInputMethod);
    bool connect();
    bool connected() const;
    bool request_uuid(std::string&);
    bool screencap(cv::Mat&);
    bool inactive();
    uint32_t window_id_ = 0;
    MaaMacOSScreencapMethod screencap_method_ = MaaMacOSScreencapMethod_None;
    MaaMacOSInputMethod input_method_ = MaaMacOSInputMethod_None;
    bool connected_ = false;
    std::shared_ptr<InputBase> input_ = nullptr;
    std::shared_ptr<ScreencapBase> screencap_ = nullptr;
};
struct NullLog {
    template<class T> const NullLog& operator<<(const T&) const noexcept { return *this; }
};
inline NullLog mfs_mac_null_log;
