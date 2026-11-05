// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
// Complete original factory/manager/NoneInput, actual virtual query and real
// caller Mat. Only the None capture method connects; no native backend or input
// action is called. The other two selectors reject before constructing input.
#include "whole-owner-observer.hpp"
namespace {
void whole_none_query() {
    for(const uint64_t method : {uint64_t{0},uint64_t{2},std::numeric_limits<uint64_t>::max()}) {
        const auto configuration=std::string("{\"screencap_method\":")+std::to_string(method)+",\"input_method\":0}";
        std::unique_ptr<MAA_CTRL_UNIT_NS::LinuxControlUnitAPI,decltype(&MaaLinuxControlUnitDestroy)> unit(
            MaaLinuxControlUnitCreate(configuration.c_str()),MaaLinuxControlUnitDestroy);
        require(bool(unit),"whole None factory rejected valid configuration");
        const auto cold=MfsWholeLinux::observe(*unit);
        require(cold.manager && !cold.capture && !cold.input && !unit->connected() && unit->get_features()==0,
            "whole None cold state differs");
        const auto info=json::value(unit->get_info()).to_string();
        const std::string expected_uuid=method==2?"_-1":"-1_-1";
        std::string uuid;require(unit->request_uuid(uuid) && uuid==expected_uuid,"whole None cold UUID differs");
        cv::Mat output(2,3,CV_8UC3,cv::Scalar(7,9,11));
        const auto* pointer=output.data;const int refs=output.u->refcount;
        const bool expected=method==0;
        require(unit->connect()==expected && unit->connected()==expected,"whole None connect result differs");
        const uint64_t features=unit->get_features();
        const auto connected=MfsWholeLinux::observe(*unit);
        require(features==(expected?3:0) && connected.manager==cold.manager,
            "whole None active features or actual owner differs");
        require(expected ? connected.capture && connected.input && connected.capture_none==1 && connected.input_none==1
            : !connected.capture && !connected.input,"whole None active dynamic types differ");
        require(unit->inactive() && unit->get_features()==features && MfsWholeLinux::observe(*unit)==connected,
            "whole None inactive changed owners/features");
        require(!unit->screencap(output) && output.data==pointer && output.u->refcount==refs,
            "whole None rejected capture changed caller storage");
        require(unit->get_features()==features && MfsWholeLinux::observe(*unit)==connected,
            "whole None rejected capture changed input owner");
        require(unit->connect()==expected && unit->connected()==expected && unit->get_features()==features,
            "whole None reconstruction changed features");
        const auto rebuilt=MfsWholeLinux::observe(*unit);
        require(rebuilt.manager==cold.manager && (expected ? rebuilt.capture_none==1 && rebuilt.input_none==1
            : !rebuilt.capture && !rebuilt.input),"whole None reconstructed dynamic types differ");
        require(json::value(unit->get_info()).to_string()==info && unit->request_uuid(uuid) && uuid==expected_uuid,
            "whole None reconstruction changed query bytes");
        unit.reset();
        require(output.data==pointer && output.u->refcount==refs,"whole None destruction changed caller owner");
        for(int y=0;y<output.rows;++y) {for(int x=0;x<output.cols;++x) {
            require(output.at<cv::Vec3b>(y,x)==cv::Vec3b(7,9,11),"whole None caller pixels changed");
        }}
        std::cout<<"whole_none method="<<method<<" cold_features=0 connected="<<expected
            <<" active_features="<<features<<" inactive_features="<<features<<" reconstructed_features="<<features
            <<" actual_capture_none="<<rebuilt.capture_none<<" actual_input_none="<<rebuilt.input_none
            <<" caller_unchanged=1 original_destroyed=1\n";
    }
    std::cout<<"Original whole Linux None manager cases=3 active_NoneInput=1 active_features=3 original_class_api=1 "
        "caller_checks=3 native_connection=0 native_captures=0 input_actions=0 full_P1_acceptance=0 passed\n";
}
}
