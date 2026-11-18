import React from "react";
import { Box, StatusIndicator } from "@cloudscape-design/components";
import { DataType } from "../../definitions/types";
import { getRunProcessingError } from "../../utils/utils";

/**
 * Placeholder box for a run's unavailable data, showing whether it was not collected
 * or is missing due to processing errors.
 */
export function DataUnavailableBox(props: { dataType: DataType; runName: string }) {
  const processingError = getRunProcessingError(props.dataType, props.runName);
  return (
    <Box textAlign="center" color="inherit">
      <b>{processingError ? "Data processing failed" : "Data not collected"}</b>
      <Box variant="p" color="inherit">
        {processingError ?? "This data was not collected in the APerf run"}
      </Box>
    </Box>
  );
}

/** Placeholder for a metric graph while its values are decoded, or if decoding failed. It is as high as a plotly.js graph. */
export function MetricDecodingStatusBox(props: { error?: Error }) {
  return (
    <div style={{ height: "450px", display: "flex", alignItems: "center", justifyContent: "center" }}>
      {props.error ? (
        <StatusIndicator type="error">{`The metric could not be decoded: ${props.error.message}`}</StatusIndicator>
      ) : (
        <StatusIndicator type="loading">Decoding the metric</StatusIndicator>
      )}
    </div>
  );
}
