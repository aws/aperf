# Development
This document contains guidelines for making certain systematic changes.

## Add a New Data Type

> [!WARNING]
> For all dependencies and implementations that are used for record (data collection) only, use the `#[cfg(target_os = "linux")]` flag to only compile them on Linux.

When APerf needs to collect data from a new source (e.g. another system pseudo file), we need to introduce a new data type. Below are the detailed steps:
1. Identify the appropriate data format that the new data type belongs to. All data formats are defined in [src/data/data_formats.rs](../src/data/data_formats.rs).
2. Create a new file `<data_name>.rs` in [src/data](../src/data) - the file name will be the data name and act as a key that APerf uses to refer the data.
3. In the new data file, define and implement two structs:
   * A struct that implements the `CollectData` trait defined in [src/data.rs](../src/data.rs). The implementation defines how the data is collected during `aperf record`. The collected raw data should be stored in the struct. The struct is serialized into a corresponding binary file at every interval, unless the data is marked as static, in which case the struct will only be written to file once. **All related implementations need to be configured to build on Linux only**.
   * A struct that implements the `ProcessData` trait defined in [src/data.rs](../src/data.rs). The implementation defines how the collected raw data are processed into one of the data formats during `aperf report`. APerf deserializes all raw data structs in the binary file, collects them into a vector, and passes it as an argument to the `process_raw_data` function of the trait.
4. In [src/data.rs](../src/data.rs), 
   * Register the new data file as a module by adding `pub mod <data_name>;` at the top.
   * Add the raw data struct to the arguments of the `data!` macro invocation.
   * Add the processed data struct to the arguments of the `report_data!` macro invocation.
5. Create unit test file `test_<data_name>.rs` in [tests/](../tests).
6. (Optional) [Create analytical rules](#add-or-update-analytical-rules) for the new data type.
7. Update [src/report_frontend/src/index.html](../src/report_frontend/src/index.html) to include the Javascript file that includes the data type's processed data.
8. Update [src/report_frontend/src/definitions/data-config.ts](../src/report_frontend/src/definitions/data-config.ts) to include the new data type name, so that the report frontend can locate its processed data and analytical findings.
9. Add the new data name to `ALL_DATA_TYPES` in [src/report_frontend/src/definitions/types.ts](../src/report_frontend/src/definitions/types.ts).
10. [Add data descriptions](#add-or-update-data-descriptions) to help users better understand the data.

## Add or Update Analytical Rules

The APerf analytical rules defined in code are applied to the processed data. If a rule matches, one or more analytical findings will be produced and shown in the report frontend. The findings can more quickly help users perform performance debugging for their service and reduce the amount of time spent on browsing through all collected data.

Below are instructions for adding or updating an analytical rule:
1. Check [src/analytics/rule_templates](../src/analytics/rule_templates) for all the analytical rule templates, which contain implementations of how a rule is matched against the processed data. Find the most appropriate template for the rule by reading the template's doc comments.
2. Every analytical rule is created out of a template, using the macro defined in the template's file, which essentially instantiates the template's struct.
3. If the rule targets a single data type, add it in the corresponding file in [src/analytics/rules](../src/analytics/rules); if the rule targets multiple data types, add it in [src/analytics/rules/multi_data_rules.rs](../src/analytics/rules/multi_data_rules.rs).
4. Every data file in [src/analytics/rules](../src/analytics/rules) implements the `AnalyzeData` trait for the processed data struct of the data, which contains one function `get_analytical_rules` that returns a vector of all analytical rules of the data.
5. Every rule needs to have a base score specified. All base score options can be found in [src/analytics/mod.rs](../src/analytics/mod.rs). To compute a finding's score, the base score will be scaled by how much the actual value deviates from the expected threshold (depending on a rule template's implementation). The finding scores will be used to sort all findings in the report. 

## Add or Update Data Descriptions

When users are browsing through the collected data, they can view the information about a specific data (e.g. metric) through the help panel. When adding a new data type, it is always recommended to add descriptions of the included data, to help users better understand how the data could impact performance.

Below are the instructions for adding or updating a data description:
1. Every data type has its own object mapped to the data name in the `DATA_DESCRIPTIONS` object in [src/report_frontend/src/definitions/data-descriptions.ts](../src/report_frontend/src/definitions/data-descriptions.ts). For a new data type, add a new entry.
2. Within a data type's object,
   * `readableName`: the human-readable name of the data type and acts as its title in the report.
   * `summary`: the description of the data type on a high level, including which part of performance it covers, how it is collected, and how to understand all the data/graphs within. The summary will be shown in the help panel when user clicks the info button by the data page's header.
   * `defaultUnit`: (for time-series data only) the unit to be used by all the metric graphs, unless a specific metric has its own unit defined (see below).
   * `defaultHelpfulLinks`: the list of helpful links to be rendered at the footer of the help panel for all of the included data.
   * `fieldDescriptions`: contains the description objects of every data in the data type (i.e. metric for time-series data, key for key-value data, graph for graph data, etc.):
     * `readableName`: the human-readable name of this data. It is used as the title of the data's help panel.
     * `description`: detailed information of this data, such as what the metric means, how it affects performance, and how to improve its value. **It is preferable to include as many details as possible**, so that users can learn the best about the data. The description will be shown in the data's help panel body.
     * `unit`: (for time-series data only) defines the unit to be used by the metric graph - it overrides the data type's `defaultUnit`.
     * `desired`: (for time-series data only) defines whether the values in the metric graph should be higher or lower to achieve a better performance. It is converted into a warning message in the data's help panel, and it also decides the color-coding of the report's statistical analysis.
     * `optimizations`: the list of optimization guides in markdown source to be rendered in the help panel. All guides are created in [src/report_frontend/src/definitions/optimization-guides.ts](src/report_frontend/src/definitions/optimization-guides.ts).
     * `helpfulLinks`: the list of additional helpful links that extend the data type's default helpful links.
