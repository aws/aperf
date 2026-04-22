import React from "react";
import { DataType, ProfilingData, GraphInfo } from "../../definitions/types";
import { PROCESSED_DATA } from "../../definitions/data-config";
import { Container, Icon, Link } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";

export interface IframeGraphProps {
  readonly dataType: DataType;
  readonly runName: string;
  readonly profilerName: string;
  readonly graphName: string;
}

export default function (props: IframeGraphProps) {
  const profilingData: ProfilingData | undefined = PROCESSED_DATA[props.dataType].runs[props.runName] as ProfilingData;
  const graphInfo: GraphInfo | undefined =
    profilingData?.profilers?.[props.profilerName]?.profiles?.[props.graphName]?.profile_graph;

  if (!graphInfo) {
    return <Container>This graph was not collected in this APerf run.</Container>;
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
