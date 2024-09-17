declare let runs_raw;
declare let system_info_raw_data;
declare let cpu_utilization_raw_data;
declare let vmstat_raw_data;
declare let kernel_config_raw_data;
declare let sysctl_raw_data;
declare let interrupts_raw_data;
declare let disk_stats_raw_data;
declare let perf_stat_raw_data;
declare let processes_raw_data;
declare let meminfo_raw_data;
declare let netstat_raw_data;
declare let perf_profile_raw_data;
declare let flamegraph_raw_data;
declare let aperf_run_stats_raw_data;
declare let java_profile_raw_data;
declare let aperf_runlog_raw_data;

let all_run_keys: Array<string> = new Array<string>();
let key_limits: Map<string, Limits> = new Map<string, Limits>();

function form_graph_limits(data) {
    key_limits.clear();
    all_run_keys.length = 0;
    for (let i = 0; i < data.runs.length; i++) {
        let key_values = data.runs[i]['key_values'];
        for (let key in key_values) {
            let metadata = JSON.parse(key_values[key])['metadata'];
            let limits = metadata.limits;
            if (key_limits.has(key)) {
                let existing_limit = key_limits.get(key);
                if (limits.low < existing_limit.low) {
                    existing_limit.low = limits.low;
                }
                if (limits.high > existing_limit.high) {
                    existing_limit.high = limits.high;
                }
            } else {
                key_limits.set(key, limits);
            }
        }
    }
    for (let i = 0; i < data.runs.length; i++) {
        let keys = data.runs[i]['keys'];
        var prev_all_run_key_index = 0;
        for (let j = 0; j < keys.length; j++) {
            let key = keys[j];
            if (all_run_keys.indexOf(key) == -1) {
                all_run_keys.splice(prev_all_run_key_index, 0, key);
            }
            prev_all_run_key_index += 1;
        }
    }

    for (let [key, value] of key_limits.entries()) {
        let extra = (value.high - value.low) * 0.1;
        value.high += extra;
        if (value.low != 0) {
            if ((value.low - extra) < 0) {
                value.low = 0;
            } else {
                value.low -= extra;
            }
        }
    }
}

function canHide(hide, keys, key) {
    let limits = key_limits.get(key);
    if (limits.low == 0 && limits.high == 0 && hide) {
        return true;
    }
    return false;
}
function emptyOrCallback(keys, hide, callback, elem, key, run_data, run="") {
    if (canHide(hide, keys, key)) {
        return;
    }
    if (keys.indexOf(key) == -1) {
        setTimeout(() => {
            emptyGraph(elem, key);
        }, 0);
    } else {
        setTimeout(() => {
            callback(elem, key, run_data[key], run);
        }, 0);
    }
}
function emptyGraph(elem, key) {
    var layout = {
        title: `${key} (N/A)`,
    }
    Plotly.newPlot(elem, [], layout, { frameMargins: 0 });
}

class RunEntry {
    run: string;
    entries: Map<string, string>;
    keys: Array<string>;
    diff_keys: Array<string>;
    raw_entries: string;
}

class Limits {
    low: number;
    high: number;
}

function clearElements(id: string) {
	let node: HTMLElement = document.getElementById(id);
	while(node.lastElementChild) {
		node.removeChild(node.lastElementChild);
	}
}

function addElemToNode(node_id: string, elem: HTMLElement) {
	let node: HTMLElement = document.getElementById(node_id);
	node.appendChild(elem);
}

function isDiffAcrossRuns(title: string, run_names, check_runs) {
    let title_value = check_runs.get(run_names[0]).entries.get(title);
    for (let [key, value] of check_runs) {
        if (title_value != value.entries.get(title)) {
            return true;
        }
    }
    return false;
}

function createNode(key, value, container_id) {
    var dt = document.createElement('dt');
    dt.innerHTML = `${key} = ${value}`;
    addElemToNode(container_id, dt);
}

function checkIsCommonKey(title: string, check_runs, check_common_keys) {
    for (let check_key of check_common_keys) {
        if (check_key == title) {
            return true;
        }
    }
    for (let [key, value] of check_runs) {
        if (value.keys.includes(title)) {
            continue;
        } else {
            return false;
        }
    }
    check_common_keys.push(title);
    return true;
}

function split_keys(check_runs, check_common_keys) {
    for (let [key, value] of check_runs) {
        let run_entry = check_runs.get(key);
        for (let title of value.keys) {
            if (!checkIsCommonKey(title, check_runs, check_common_keys)) {
                run_entry.diff_keys.push(title);
            }
        }
    }
}

function allRunCPUListUnchanged(cpu_list) {
    for (let i = 0; i < runs_raw.length; i++) {
        let run_name = runs_raw[i];
        if (!isCPUListUnchanged(cpu_list.get(run_name).cpulist, run_name)) {
            return false;
        }
    }
    return true;
}
