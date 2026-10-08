import React from "react";
import { Box } from "@cloudscape-design/components";
import { DataType } from "../../definitions/types";
import { getRunProcessingError } from "../../utils/utils";

/**
 * Placeholder box for a run's unavailable data, showing whether it was not collected
 * or is missing due to processing errors.
 */
export default function (props: { dataType: DataType; runName: string }) {
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
