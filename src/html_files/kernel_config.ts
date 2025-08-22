let got_kernel_config_data = false;
let current_kernel_diff_status = false;

var kernel_config_runs: Map<string, RunEntry> = new Map<string, RunEntry>();
var kernel_config_run_names: Array<string> = [];
var kernel_config_common_keys: Array<string> = [];

function createEntries(container_id, values, level, run) {
    values?.forEach(function(value, index, arr) {
        for (var prop in value) {
            if ('value' in value[prop]) {
                var dt = document.createElement('dt');
                dt.style.textIndent = `${level * 5}%`;
                dt.style.fontWeight = "normal";
                dt.innerHTML = `${value[prop].name} = ${value[prop].value}`;
                addElemToNode(container_id, dt);
            } else {
                var dl = document.createElement('dl');
                dl.style.textIndent = `${level * 5}%`;
                dl.innerHTML = value[prop].name;
                dl.style.fontWeight = "bold";
                dl.id = `${run}-${value[prop].name}`;
                addElemToNode(container_id, dl);
                setTimeout(() => {
                    createEntries(dl.id, value[prop].entries, level + 1, run);
                }, 0);
            }
        }
    });
}

function form_entries_keys(entries, run) {
    entries.forEach(function(value) {
        for (var prop in value) {
            if ('value' in value[prop]) {
                let run_entry = kernel_config_runs.get(run);
                let title = value[prop].name;
                let title_value = value[prop].value;
                run_entry.entries.set(title, title_value);
                run_entry.keys.push(title);
                if (kernel_config_common_keys.indexOf(value[prop].name) != -1) {
                    kernel_config_common_keys.push(value[prop].name);
                }
            } else if ('entries' in value[prop]) {
                form_entries_keys(value[prop].entries, run);
            }
        }
    });
}

function form_kernel_data(run, run_data) {
    kernel_config_run_names.push(run);
    var run_entry = new RunEntry();
    run_entry.run = run;
    run_entry.entries = new Map<string, string>();
    run_entry.keys = new Array();
    run_entry.diff_keys = new Array();
    run_entry.raw_entries = run_data;
    kernel_config_runs.set(run, run_entry);
    let data = JSON.parse(run_entry.raw_entries['key_values']['values']);
    data.forEach(function (value, index, arr) {
        form_entries_keys(value.entries, run);
    });
}

function kernelConfigNoDiff(run, container_id) {
    let run_entry = kernel_config_runs.get(run);
    let data = JSON.parse(run_entry.raw_entries['key_values']['values']);
    var dl = document.createElement('dl');
    dl.id = `${run}-dl-kernel-config`;
    dl.classList.add("extra");
    dl.style.float = "none";
    var dl_id = dl.id;
    addElemToNode(container_id, dl);
    data.forEach(function (value, index, arr) {
        var dt = document.createElement('dl');
        dt.id = `${run_entry.run}-${value.name}`;
        dt.style.fontWeight = "bold";
        dt.innerHTML = value.name;
        addElemToNode(dl_id, dt);
        createEntries(dt.id, value.entries, 1, run_entry.run);
    });
}

function kernelConfigDiff(value, container_id) {
    var dl = document.createElement('dl');
    dl.id = `${value}-dl-kernel-config`;
    dl.classList.add("extra");
    dl.style.float = "none";
    var dl_id = dl.id;
    addElemToNode(container_id, dl);
    let run_entry = kernel_config_runs.get(value);
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
}

function kernelConfig(diff: boolean) {
    if (got_kernel_config_data && current_kernel_diff_status == diff) {
        return;
    }
    current_kernel_diff_status = diff;
    clear_and_create('kernel');

    let no_data_run_names = new Set();
    for (let run of kernel_config_raw_data['runs']) {
        let run_name = run["name"];
        let elem_id = `${run_name}-kernel-per-data`;
        if (handleNoData(elem_id, run["key_values"])) {
            no_data_run_names.add(run_name);
        }
    }

    var data = runs_raw;
    if (!got_kernel_config_data) {
        data.forEach(function (value, index, arr) {
            if (no_data_run_names.has(value)) return;

            let this_run_data;
            for (let i = 0; i < kernel_config_raw_data['runs'].length; i++) {
                if (kernel_config_raw_data['runs'][i]['name'] == value) {
                    this_run_data = kernel_config_raw_data['runs'][i];
                    form_kernel_data(value, this_run_data);
                }
            }
        })
        split_keys(kernel_config_runs, kernel_config_common_keys);
    }

    data.forEach(function (value, index, arr) {
        if (no_data_run_names.has(value)) return;

        let elem_id = `${value}-kernel-per-data`;
        if (current_kernel_diff_status) {
            kernelConfigDiff(value, elem_id);
        } else {
            kernelConfigNoDiff(value, elem_id);
        }
    })
    got_kernel_config_data = true;
}
