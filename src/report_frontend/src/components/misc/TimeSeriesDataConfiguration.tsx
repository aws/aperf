import React from "react";
import { CollectionPreferences, Multiselect, SpaceBetween } from "@cloudscape-design/components";
import { useReportState } from "../ReportStateProvider";
import { MAX_NUM_CPU_SHOW_DEFAULT, NUM_METRICS_PER_PAGE } from "../../definitions/constants";
import { RUNS } from "../../definitions/data-config";
import { NumCpusPerRun, SelectedCpusPerRun } from "../../definitions/types";
import { SelectProps } from "@cloudscape-design/components/select/interfaces";

type SelectedCpuOptionsPerRun = { [key in string]: SelectProps.Options };
type SelectedCpuOptionsUpdate = { runName: string; selectedCpuOptions: SelectProps.Options };

const selectedCpuOptionsReducer = (
  curSelectedCpusPerRun: SelectedCpuOptionsPerRun,
  selectedCpusUpdate: SelectedCpuOptionsUpdate,
) => ({
  ...curSelectedCpusPerRun,
  [selectedCpusUpdate.runName]: selectedCpusUpdate.selectedCpuOptions,
});

function getCpuOption(cpuNumber: number): SelectProps.Option {
  return cpuNumber < 0
    ? { label: "Aggregate", value: "-1" }
    : { label: `CPU${cpuNumber}`, value: cpuNumber.toString() };
}

function getAllCpuOptionsPerRun(numCpusPerRun: NumCpusPerRun): Map<string, SelectProps.Option[]> {
  const allCpuOptions = new Map<string, SelectProps.Option[]>();
  for (const runName in numCpusPerRun) {
    const curRunCpuOptions: SelectProps.Option[] = [getCpuOption(-1)];
    for (let i = 0; i < numCpusPerRun[runName]; i++) {
      curRunCpuOptions.push(getCpuOption(i));
    }
    allCpuOptions.set(runName, curRunCpuOptions);
  }
  return allCpuOptions;
}

/**
 * Converts currently selected CPUs into the options to be shown in the configuration page -
 * used when the user opens up the configuration modal
 */
function selectedCpusToOptions(
  selectedCpusPerRun: SelectedCpusPerRun,
  updateSelectedCpuOptions: React.ActionDispatch<[selectedCpusUpdate: SelectedCpuOptionsUpdate]>,
) {
  for (const runName in selectedCpusPerRun) {
    const curRunCpuOptions: SelectProps.Option[] = [];
    if (selectedCpusPerRun[runName].aggregate) curRunCpuOptions.push(getCpuOption(-1));
    selectedCpusPerRun[runName].cpus.forEach((selected: boolean, cpu: number) => {
      if (selected) {
        curRunCpuOptions.push(getCpuOption(cpu));
      }
    });
    updateSelectedCpuOptions({
      runName,
      selectedCpuOptions: curRunCpuOptions,
    });
  }
}

/**
 * Converts the selected options into the actual selected CPUs to be consumed by metric graphs -
 * used when the user clicks confirm and closes the configuration modal
 */
function optionsToSelectedCpus(
  selectedCpuOptionsPerRun: SelectedCpuOptionsPerRun,
  numCpusPerRun: NumCpusPerRun,
  setSelectedCpusPerRun: (newSelectedCpusPerRun: SelectedCpusPerRun) => void,
) {
  const selectedCpusPerRun: SelectedCpusPerRun = {};
  for (const runName in selectedCpuOptionsPerRun) {
    let selectedAggregate = false;
    const selectedCpus: boolean[] = Array(numCpusPerRun[runName]).fill(false);
    for (const selectedOption of selectedCpuOptionsPerRun[runName]) {
      if (selectedOption.label == "Aggregate") {
        selectedAggregate = true;
      } else {
        selectedCpus[Number(selectedOption.value)] = true;
      }
    }
    selectedCpusPerRun[runName] = {
      aggregate: selectedAggregate,
      cpus: selectedCpus,
    };
  }
  setSelectedCpusPerRun(selectedCpusPerRun);
}

/**
 * This component renders the configuration for time series data, which allows users to select
 * the number of graphs to render on each page as well as the visible CPUs
 */
export default function () {
  const {
    numMetricGraphsPerPage,
    setNumMetricGraphsPerPage,
    numCpusPerRun,
    selectedCpusPerRun,
    setSelectedCpusPerRun,
  } = useReportState();

  const allCpuOptionsPerRun = React.useMemo(() => getAllCpuOptionsPerRun(numCpusPerRun), [numCpusPerRun]);

  const [selectedCpuOptionsPerRun, updateSelectedCpuOptions] = React.useReducer(selectedCpuOptionsReducer, {});
  React.useEffect(() => {
    selectedCpusToOptions(selectedCpusPerRun, updateSelectedCpuOptions);
  }, [selectedCpusPerRun]);

  return (
    <CollectionPreferences
      title={"Configuration"}
      confirmLabel={"Confirm"}
      cancelLabel={"Cancel"}
      preferences={{
        pageSize: numMetricGraphsPerPage,
        custom: {
          cpu_config: true,
        },
      }}
      pageSizePreference={{
        title: "Number of Metric Graphs per Page",
        options: [
          {
            value: NUM_METRICS_PER_PAGE,
            label: `${NUM_METRICS_PER_PAGE} graphs (default)`,
          },
          {
            value: 30,
            label: "30 graphs",
          },
          {
            value: 60,
            label: "60 graphs",
          },
        ],
      }}
      customPreference={() => (
        <SpaceBetween size={"s"}>
          {RUNS.map((runName) => (
            <Multiselect
              enableSelectAll
              expandToViewport
              placeholder={`Choose visible CPUs for run ${runName}`}
              i18nStrings={{
                selectAllText: "Select all",
                tokenLimitShowMore: "Show more",
                tokenLimitShowFewer: "Show fewer",
              }}
              tokenLimit={MAX_NUM_CPU_SHOW_DEFAULT}
              selectedOptions={selectedCpuOptionsPerRun[runName]}
              options={allCpuOptionsPerRun.get(runName)}
              onChange={({ detail }) => {
                updateSelectedCpuOptions({ runName, selectedCpuOptions: detail.selectedOptions });
              }}
            />
          ))}
        </SpaceBetween>
      )}
      onCancel={() => selectedCpusToOptions(selectedCpusPerRun, updateSelectedCpuOptions)}
      onConfirm={({ detail }) => {
        setNumMetricGraphsPerPage(detail.pageSize);
        optionsToSelectedCpus(selectedCpuOptionsPerRun, numCpusPerRun, setSelectedCpusPerRun);
      }}
    />
  );
}
