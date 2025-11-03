import React from "react";
import { DataType, GraphData, GraphInfo } from "../../definitions/types";
import { PROCESSED_DATA } from "../../definitions/data-config";
import { Container, Icon, Link } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";

export interface IframeGraphProps {
  readonly dataType: DataType;
  readonly runName: string;
  readonly graphGroup: string;
  readonly graphName: string;
}

export default function (props: IframeGraphProps) {
  const graphData: GraphData | undefined = PROCESSED_DATA[props.dataType].runs[props.runName] as GraphData;
  const graphInfo: GraphInfo | undefined = graphData?.graph_groups[props.graphGroup]?.graphs[props.graphName];

  if (!graphInfo) {
    return <Container>This data was not collected in the APerf run.</Container>;
  } else {
    return (
      <Container
        header={
          <Header
            actions={
              <Link href={graphInfo.graph_path} target={"_blank"} rel={"noopener noreferrer"}>
                <Icon name={"expand"} />
              </Link>
            }
          />
        }
        media={{
          content: <iframe style={{ height: "100vh", width: "100%" }} src={graphInfo.graph_path} />,
        }}
      />
    );
  }
}
