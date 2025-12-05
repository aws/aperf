import React from "react";
import { SideNavigation, SideNavigationProps, Box } from "@cloudscape-design/components";
import { APERF_SERVICE_NAME } from "../../definitions/constants";
import { NAVIGATION_CONFIGS, VERSION_INFO } from "../../definitions/data-config";
import { DataType } from "../../definitions/types";
import { useReportState } from "../ReportStateProvider";
import { extractDataTypeFromFragment } from "../../utils/utils";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";

/**
 * This component renders the navigation panel on the left side that allows users to navigate between
 * different data
 */
export default function () {
  const { setDataComponent, setSearchKey } = useReportState();

  const items: SideNavigationProps.Item[] = [
    { type: "link", text: DATA_DESCRIPTIONS["systeminfo"].readableName, href: "#systeminfo" },
  ];
  NAVIGATION_CONFIGS.forEach((section) => {
    const sectionItems = section.items.map(
      (dataType: DataType) =>
        ({
          type: "link",
          text: DATA_DESCRIPTIONS[dataType].readableName,
          href: `#${dataType}`,
        }) as SideNavigationProps.Link,
    );
    items.push({
      type: "section",
      text: section.sectionName,
      items: sectionItems,
    } as SideNavigationProps.Section);
  });
  items.push({ type: "divider" });
  items.push({ type: "link", text: "GitHub Repository", href: "https://github.com/aws/aperf", external: true });
  items.push({
    type: "link",
    text: "Leave us your feedback",
    href: "https://github.com/aws/aperf/discussions/329",
    external: true,
  });
  items.push({ type: "divider" });

  return (
    <>
      <SideNavigation
        header={{
          href: "",
          text: APERF_SERVICE_NAME,
        }}
        items={items}
        onFollow={(event) => {
          setSearchKey("");
          const dataType = extractDataTypeFromFragment(event.detail.href);
          if (dataType) setDataComponent(dataType);
          else setDataComponent("systeminfo");
        }}
      />
      <Box color="text-body-secondary" padding={{ left: "xl" }}>
        <strong>Version Info</strong>
        <br />
        Cargo Version: {VERSION_INFO.version}
        <br />
        Git SHA: {VERSION_INFO.git_sha.substring(0, 8)}
      </Box>
    </>
  );
}
