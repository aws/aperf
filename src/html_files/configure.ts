class RunConfig {
    run: string;
    cpu_count: number;
    all_selected: boolean;
    cpu_list: Array<string>;
    x_range: Array<number>;
    x_default_range: Array<number>;
}
class CPUList {
    all_selected: boolean;
    cpulist: Array<string>;
}
var run_config: Map<string, RunConfig> = new Map<string, RunConfig>();
var init_done = false;

function getXRange(run_name) {
    return run_config.get(run_name).x_range.slice();
}

function xLimitsUnchanged(run_name, curr_range) {
    let run_range = run_config.get(run_name).x_range;
    return (curr_range[0] == run_range[0]) && (curr_range[1] == run_range[1]);
}

function getCPUList(run) {
    let ret = new CPUList();
    ret.all_selected = run_config.get(run).all_selected;
    ret.cpulist = run_config.get(run).cpu_list.slice();
    return ret;
}

function formGlobalConfig() {
    for (let i = 0; i < cpu_utilization_raw_data['runs'].length; i++) {
        let this_run_data = cpu_utilization_raw_data['runs'][i];
        let run_name = this_run_data['name'];
        var config = new RunConfig();

        /* Elemet 0 is aggregate. Don't use that. */
        let key = this_run_data['keys'][1];
        config.cpu_count = JSON.parse(this_run_data['key_values'][key]).length;
        config.cpu_list = new Array<string>();
        for (let i = 0; i < config.cpu_count; i++) {
            config.cpu_list.push(i.toString());
        }
        config.all_selected = true;
        run_config.set(run_name, config);
        var x_end = JSON.parse(this_run_data['key_values']['aggregate']).at(-1).time.TimeDiff;
        config.x_range = [0, x_end];
        config.x_default_range = [0, x_end];
    }
}

function isCPUListUnchanged(check_cpu_list, run) {
    return check_cpu_list.sort().toString() === run_config.get(run).cpu_list.sort().toString();
}

function allSelect(elem: HTMLInputElement, run) {
    let checkboxes = document.getElementsByName(`${run}-cpulist`);
    for (let i = 0; i < checkboxes.length; i++) {
        (checkboxes[i] as HTMLInputElement).checked = elem.checked;
        toggleCheckbox(checkboxes[i] as HTMLInputElement, run, false);
    }
}

function allCPUsCheck(run) {
    let checkboxes = document.getElementsByName(`${run}-cpulist`);
    let allToggled = true;
    for (let i = 0; i < checkboxes.length; i++) {
        if (!((checkboxes[i] as HTMLInputElement).checked)) {
            allToggled = false;
            break;
        }
    }
    (document.getElementById(`${run}-CPUAll`) as HTMLInputElement).checked = allToggled;
    run_config.get(run).all_selected = allToggled;
}

function toggleCheckbox(elem: HTMLInputElement, run, check) {
    let cpu_list = run_config.get(run).cpu_list;
    if (elem.checked) {
        if (cpu_list.indexOf(elem.value) == -1) {
            cpu_list.push(elem.value);
        }
    } else {
        let index = cpu_list.indexOf(elem.value);
        if (index != -1) {
            cpu_list.splice(index, 1);
        }
    }
    if (check) {
        allCPUsCheck(run);
    }
}

function createCheckbox(run, id, label_str, name, value, toggle_func) {
    let elem = document.createElement("input");
    elem.setAttribute("type", "checkbox");
    elem.id = id;
    elem.name = name;
    elem.value = value;
    elem.addEventListener("click", function (ev: Event) {
        toggle_func(this, run, true);
    }, false);
    let label = document.createElement("label");
    label.htmlFor = id;
    label.innerHTML = label_str;

    let cpu_div = document.createElement("div");
    cpu_div.appendChild(elem);
    cpu_div.appendChild(label);
    return cpu_div;
}

function createCPUConfigure(container_id, run) {
    let config = run_config.get(run);

    /* Add a Toggle All checkbox */
    let all_cpu = createCheckbox(run, `${run}-CPUAll`, 'Toggle All', '', -1, allSelect);
    addElemToNode(container_id, all_cpu);

    let cpu_configure_list = document.createElement("div");
    cpu_configure_list.id = `${run}-cpus-configure-list`;
    cpu_configure_list.style.display = "grid";
    addElemToNode(container_id, cpu_configure_list);
    let max_cpus = config.cpu_count;
    let max_columns = max_cpus / 16;
    let cpus_per_column = Math.floor(max_cpus / max_columns);
    for (let column = 0; column < max_columns; column++) {
        let cpus_start = column * cpus_per_column;
        let cpus_end = cpus_start + cpus_per_column;
        if (cpus_end > max_cpus) {
            cpus_end = max_cpus;
        }
        for (let i = cpus_start, row = 1; i < cpus_end; i++, row++) {
            let cpu_div = createCheckbox(run, `${run}-CPU${i}`, `CPU ${i}`, `${run}-cpulist`, i, toggleCheckbox);
            cpu_div.style.gridColumn = `${column + 1}`;
            cpu_div.style.gridRow = `${row}`;
            addElemToNode(`${run}-cpus-configure-list`, cpu_div);
        }
    }
    document.getElementById(`${run}-CPUAll`).click();
}

function configure() {
    if (init_done) {
        return;
    }
    clear_and_create('configure');
    for (let i = 0; i < cpu_utilization_raw_data['runs'].length; i++) {
        let run_name = cpu_utilization_raw_data['runs'][i]['name'];
        let this_run_data = cpu_utilization_raw_data['runs'][i];
        let x_selector: any = document.createElement("div");
        x_selector.id = `${run_name}-x-selector`;
        x_selector.dataset.run_name = run_name;
        let per_id = `${run_name}-configure-per-data`;
        addElemToNode(per_id, x_selector);
        createCPUConfigure(per_id, run_name);
        getCpuUtilization(run_name, x_selector, this_run_data['key_values']['aggregate'], true);
        x_selector.on('plotly_relayout', function(eventdata) {
            var x_start = 0, x_end = 0;
            if ('xaxis.autorange' in eventdata) {
                // Handle double click to reset.
                var x_range = run_config.get(x_selector.dataset.run_name).x_default_range;
                x_start = x_range[0];
                x_end = x_range[1];
            } else if ('xaxis.range[0]' in eventdata) {
                // Handle using the big graph to configure the x axis.
                x_start = eventdata['xaxis.range[0]'];
                x_end = eventdata['xaxis.range[1]'];
            } else {
                // Handle using the rangeslider to configure the x axis.
                x_start = eventdata['xaxis.range'][0];
                x_end = eventdata['xaxis.range'][1];
            }
            run_config.get(x_selector.dataset.run_name).x_range = [x_start, x_end];
        });
    }
    init_done = true;
}
