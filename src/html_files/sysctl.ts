let got_sysctl_data = false;
let current_sysctl_diff_status = false;

var sysctl_runs: Map<string, RunEntry> = new Map<string, RunEntry>();
var sysctl_run_names: Array<string> = [];
var sysctl_common_keys: Array<string> = [];

function form_sysctl_data(run, run_data) {
    sysctl_run_names.push(run);
    var run_entry = new RunEntry();
    run_entry.run = run;
    run_entry.entries = new Map<string, string>();
    run_entry.keys = new Array();
    run_entry.diff_keys = new Array();
    sysctl_runs.set(run, run_entry);
    var data = JSON.parse(run_data['key_values']['values']);
    for (var key in data) {
        var value = data[key];
        let run_entry = sysctl_runs.get(run);
        run_entry.entries.set(key, value);
        run_entry.keys.push(key);
    }
}
function sysctlNoDiff(run, container_id) {
    var dl = document.createElement('dl');
    dl.id = `${run}-dl-sysctl-data`;
    dl.classList.add("extra");
    dl.style.float = "none";
    var dl_id = dl.id;
    addElemToNode(container_id, dl);
    let run_entry = sysctl_runs.get(run);
    for (let [key, value] of run_entry.entries) {
        createNode(key, value, dl_id);
    }
}

function sysctlDiff(value) {
    var agg_id = `${value}-sysctl-data-div`;
    clearElements(agg_id);
    var dl = document.createElement('dl');
    dl.id = `${value}-dl-sysctl-data`;
    dl.classList.add("extra");
    dl.style.float = "none";
    var dl_id = dl.id;
    addElemToNode(agg_id, dl);
    let run_entry = sysctl_runs.get(value);
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
}

function sysctl(diff: boolean) {
    if (got_sysctl_data && current_sysctl_diff_status == diff) {
        return;
    }
    current_sysctl_diff_status = diff;
    var data = runs_raw;
    if (!got_sysctl_data) {
        data.forEach(function (value, index, arr) {
            let this_run_data;
            for (let i = 0; i < sysctl_raw_data['runs'].length; i++) {
                if (sysctl_raw_data['runs'][i]['name'] == value) {
                    this_run_data = sysctl_raw_data['runs'][i];
                    form_sysctl_data(value, this_run_data);
                }
            }
        });
        split_keys(sysctl_runs, sysctl_common_keys);
    }

    clearElements('sysctl-data-runs');
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        run_div.id = `${value}-sysctl-data`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('sysctl-data-runs', run_div);
        var run_node_id = run_div.id;

        //Show aggregate data
        var agg_elem = document.createElement('div');
        agg_elem.id = `${value}-sysctl-data-div`;
        addElemToNode(run_node_id, agg_elem);

        if (current_sysctl_diff_status) {
            sysctlDiff(value);
        } else {
            sysctlNoDiff(value, agg_elem.id);
        }
    })
    got_sysctl_data = true;
}