let got_kernel_config_data = false;
let current_diff_status = false;

var kernel_config_runs: Map<string, RunEntry> = new Map<string, RunEntry>();
var kernel_config_run_names: Array<string> = [];
var kernel_config_common_keys: Array<string> = [];

function createEntries(container_id, values, level, run, store) {
    values?.forEach(function(value, index, arr) {
        for (var prop in value) {
            if ('value' in value[prop]) {
                var dt = document.createElement('dt');
                dt.style.textIndent = `${level * 5}%`;
                dt.style.fontWeight = "normal";
                dt.innerHTML = `${value[prop].name} = ${value[prop].value}`;
                addElemToNode(container_id, dt);
                if (store) {
                    let run_entry = kernel_config_runs.get(run);
                    let title = value[prop].name;
                    let title_value = value[prop].value;
                    run_entry.entries.set(title, title_value);
                    run_entry.keys.push(title);
                }
            } else {
                var dl = document.createElement('dl');
                dl.style.textIndent = `${level * 5}%`;
                dl.innerHTML = value[prop].name;
                dl.style.fontWeight = "bold";
                dl.id = `${run}-${value[prop].name}`;
                addElemToNode(container_id, dl);
                setTimeout(() => {
                    createEntries(dl.id, value[prop].entries, level + 1, run, store);
                }, 0);
            }
        }
    });
}

function getKernelConfig(run, container_id, run_data, diff) {
    kernel_config_run_names.push(run);
    var run_entry = new RunEntry();
    run_entry.run = run;
    run_entry.entries = new Map<string, string>();
    run_entry.keys = new Array();
    run_entry.diff_keys = new Array();
    kernel_config_runs.set(run, run_entry);
    var data = JSON.parse(run_data);
    var dl = document.createElement('dl');
    dl.id = `${run}-dl-kernel-config`;
    dl.style.float = "none";
    var dl_id = dl.id;
    addElemToNode(container_id, dl);
    kernel_config_runs.get(run).raw_entries = run_data;
    data.forEach(function (value, index, arr) {
        var dt = document.createElement('dl');
        dt.id = `${run}-${value.name}`;
        dt.style.fontWeight = "bold";
        dt.innerHTML = value.name;
        addElemToNode(dl_id, dt);
        createEntries(dt.id, value.entries, 1, run, true);
    });
}

function redoKernelConfig(diff) {
    kernel_config_run_names.forEach(function (value, index, array) {
        var agg_id = `${value}-kernel-config-div`;
        clearElements(agg_id);
        var dl = document.createElement('dl');
        dl.id = `${value}-dl-kernel-config`;
        dl.style.float = "none";
        var dl_id = dl.id;
        addElemToNode(agg_id, dl);
        let run_entry = kernel_config_runs.get(value);
        if (diff) {
            var h3_common = document.createElement('h3');
            h3_common.innerHTML = 'Common Keys';
            h3_common.style.textAlign = "center";
            addElemToNode(dl_id, h3_common);
            for (let i = 0; i < kernel_config_common_keys.length; i++) {
                if (isDiffAcrossRuns(kernel_config_common_keys[i], kernel_config_run_names, kernel_config_runs)) {
                    let e = run_entry.entries.get(kernel_config_common_keys[i]);
                    createNode(kernel_config_common_keys[i], e, dl_id);
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
            let data = JSON.parse(run_entry.raw_entries);
            data.forEach(function (value, index, arr) {
                var dt = document.createElement('dl');
                dt.id = `${run_entry.run}-${value.name}`;
                dt.style.fontWeight = "bold";
                dt.innerHTML = value.name;
                addElemToNode(dl_id, dt);
                createEntries(dt.id, value.entries, 1, run_entry.run, false);
            });
        }
    })
}

function kernelConfig(diff: boolean) {
    if (got_kernel_config_data) {
        if (current_diff_status != diff) {
            current_diff_status = diff;
            redoKernelConfig(diff);
        }
        return;
    }
    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('kernel-config-runs');
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-kernel-config`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('kernel-config-runs', run_div);
        var run_node_id = run_div.id;

        // Run name
        var h3_run_name = document.createElement('h3');
        h3_run_name.innerHTML = `${value}`;
        h3_run_name.style.textAlign = "center";
        addElemToNode(run_node_id, h3_run_name);

        //Show aggregate data
        var agg_elem = document.createElement('div');
        agg_elem.id = `${value}-kernel-config-div`;
        addElemToNode(run_node_id, agg_elem);
        for (let i = 0; i < kernel_config_raw_data['runs'].length; i++) {
            if (kernel_config_raw_data['runs'][i]['name'] == value) {
                this_run_data = kernel_config_raw_data['runs'][i];
            }
        }
        getKernelConfig(value, agg_elem.id, this_run_data['key_values']['values'], diff);
    })
    split_keys(kernel_config_runs, kernel_config_common_keys);
    got_kernel_config_data = true;
}
