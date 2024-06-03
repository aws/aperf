class RunConfig {
    run: string;
    cpu_count: number;
    all_selected: boolean;
    cpu_list: Array<string>;
}
class CPUList {
    all_selected: boolean;
    cpulist: Array<string>;
}
var run_config: Map<string, RunConfig> = new Map<string, RunConfig>();
var init_done = false;

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
    runs_raw.forEach(function (value, index, arr) {
        createCPUConfigure(`${value}-configure-per-data`, value);
    });
    init_done = true;
}
