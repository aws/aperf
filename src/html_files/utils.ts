declare let runs_raw;
declare let system_info_raw_data;
declare let cpu_utilization_raw_data;
declare let vmstat_raw_data;
declare let kernel_config_raw_data;
declare let sysctl_raw_data;
declare let interrupts_raw_data;
declare let disk_stats_raw_data;
declare let perf_stat_raw_data;

class RunEntry {
    run: string;
    entries: Map<string, string>;
    keys: Array<string>;
    diff_keys: Array<string>;
    raw_entries: string;
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
        if (value.entries.has(title)) {
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
