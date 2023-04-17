import { clearElements, addElemToNode } from "./index.js";
export { sysctl };

let got_data = false;
let current_diff_status = false;

var runs: Map<string, RunEntry> = new Map<string, RunEntry>();
var run_names: Array<string> = [];
var common_keys: Array<string> = [];

class RunEntry {
    run: string;
    entries: Map<string, string>;
    diff_keys: Array<string>;
}

function checkIfCommonKey(title: string) {
    for (let check_key of common_keys) {
        if (check_key == title) {
            return true;
        }
    }
    let common_entry_across_runs = true;
    for (let [key, value] of runs) {
        if (value.entries.has(title)) {
            continue;
        } else {
            common_entry_across_runs = false;
            return false;
        }
    }
    if (common_entry_across_runs) {
        for (let [key, value] of runs) {
            for (let i = 0; i < value.diff_keys.length; i++) {
                if (value.diff_keys[i] == title) {
                    value.diff_keys.splice(i, 1);
                }
            }
        }
        common_keys.push(title);
        return true;
    }
}

function isDiffAcrossRuns(title: string) {
    let title_value = runs.get(run_names[0]).entries.get(title);
    for (let [key, value] of runs) {
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

function getSysctl(run, container_id) {
    const http = new XMLHttpRequest();
    run_names.push(run);
    var run_entry = new RunEntry();
    run_entry.run = run;
    run_entry.entries = new Map<string, string>();
    run_entry.diff_keys = new Array();
    runs.set(run, run_entry);
    http.onload = function () {
        var data = JSON.parse(http.response);
        var dl = document.createElement('dl');
        dl.id = `${run}-dl-sysctl-data`;
        dl.style.float = "none";
        var dl_id = dl.id;
        addElemToNode(container_id, dl);
        for (var key in data) {
            var value = data[key];
            let run_entry = runs.get(run);
            run_entry.entries.set(key, value);
            if (checkIfCommonKey(key)) {
                for (let i = 0; i < run_entry.diff_keys.length; i++) {
                    if (run_entry.diff_keys[i] == key) {
                        run_entry.diff_keys.splice(i, 1);
                        break;
                    }
                }
            } else {
                run_entry.diff_keys.push(key);
            }
            createNode(key, value, dl_id);
        }
    }
    http.open("GET", `visualize/sysctl?run=${run}&get=values`);
    http.send();
}

function redoSysctl(diff: boolean) {
    run_names.forEach(function (value, index, array) {
        var agg_id = `${value}-sysctl-data-div`;
        clearElements(agg_id);
        var dl = document.createElement('dl');
        dl.id = `${value}-dl-sysctl-data`;
        dl.style.float = "none";
        var dl_id = dl.id;
        addElemToNode(agg_id, dl);
        let run_entry = runs.get(value);
        if (diff) {
            var h3_common = document.createElement('h3');
            h3_common.innerHTML = 'Common Keys';
            h3_common.style.textAlign = "center";
            addElemToNode(dl_id, h3_common);
            for (let i = 0; i < common_keys.length; i++) {
                if (isDiffAcrossRuns(common_keys[i])) {
                    let e = run_entry.entries.get(common_keys[i]);
                    createNode(common_keys[i], e, dl_id);
                }
            }
            var h3_diff = document.createElement('h3');
            h3_diff.innerHTML = 'Different Keys';
            h3_diff.style.textAlign = "center";
            addElemToNode(dl_id, h3_diff);
            for (let i = 0; i < run_entry.diff_keys.length; i++) {
                let key = run_entry.diff_keys[i];
                let e = run_entry.entries.get(key);
                createNode(key, e, dl_id);
            }
        } else {
            for (let [key, value] of run_entry.entries) {
                createNode(key, value, dl_id);
            }
        }
    })
}

function sysctl(diff: boolean) {
    if (got_data) {
        if (current_diff_status != diff) {
            current_diff_status = diff;
            redoSysctl(diff);
        }
        return;
    }

    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('sysctl-data-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-sysctl-data`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('sysctl-data-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = `${value}`;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            //Show aggregate data
            var agg_elem = document.createElement('div');
            agg_elem.id = `${value}-sysctl-data-div`;
            addElemToNode(run_node_id, agg_elem);
            getSysctl(value, agg_elem.id);
        })
        got_data = true;
    }
    http.open("GET", 'visualize/get?get=entries');
    http.send();
}