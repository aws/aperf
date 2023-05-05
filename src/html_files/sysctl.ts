let got_sysctl_data = false;
let current_sysctl_diff_status = false;

var sysctl_runs: Map<string, RunEntry> = new Map<string, RunEntry>();
var sysctl_run_names: Array<string> = [];
var sysctl_common_keys: Array<string> = [];

function getSysctl(run, container_id, run_data) {
    const http = new XMLHttpRequest();
    sysctl_run_names.push(run);
    var run_entry = new RunEntry();
    run_entry.run = run;
    run_entry.entries = new Map<string, string>();
    run_entry.keys = new Array();
    run_entry.diff_keys = new Array();
    sysctl_runs.set(run, run_entry);
    var data = JSON.parse(run_data);
    var dl = document.createElement('dl');
    dl.id = `${run}-dl-sysctl-data`;
    dl.style.float = "none";
    var dl_id = dl.id;
    addElemToNode(container_id, dl);
    for (var key in data) {
        var value = data[key];
        let run_entry = sysctl_runs.get(run);
        run_entry.entries.set(key, value);
        run_entry.keys.push(key);
        createNode(key, value, dl_id);
    }
}

function redoSysctl(diff: boolean) {
    sysctl_run_names.forEach(function (value, index, array) {
        var agg_id = `${value}-sysctl-data-div`;
        clearElements(agg_id);
        var dl = document.createElement('dl');
        dl.id = `${value}-dl-sysctl-data`;
        dl.style.float = "none";
        var dl_id = dl.id;
        addElemToNode(agg_id, dl);
        let run_entry = sysctl_runs.get(value);
        if (diff) {
            var h3_common = document.createElement('h3');
            h3_common.innerHTML = 'Common Keys';
            h3_common.style.textAlign = "center";
            addElemToNode(dl_id, h3_common);
            for (let i = 0; i < sysctl_common_keys.length; i++) {
                if (isDiffAcrossRuns(sysctl_common_keys[i], sysctl_run_names, sysctl_runs)) {
                    let e = run_entry.entries.get(sysctl_common_keys[i]);
                    createNode(sysctl_common_keys[i], e, dl_id);
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
    if (got_sysctl_data) {
        if (current_sysctl_diff_status != diff) {
            current_sysctl_diff_status = diff;
            redoSysctl(diff);
        }
        return;
    }

    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('sysctl-data-runs');
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
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
        for (let i = 0; i < sysctl_raw_data['runs'].length; i++) {
            if (sysctl_raw_data['runs'][i]['name'] == value) {
                this_run_data = sysctl_raw_data['runs'][i];
            }
        }
        getSysctl(value, agg_elem.id, this_run_data['key_values']['values']);
    })
    split_keys(sysctl_runs, sysctl_common_keys);
    got_sysctl_data = true;
}