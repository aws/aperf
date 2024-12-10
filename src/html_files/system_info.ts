let got_system_info_data = false;
let system_info_rules = {
    data_type: "system_info",
    pretty_name: "System Info",
    rules: [
        {
            name: "System Name",
            func: function (ruleOpts: RuleOpts) {
                let os_version = get_data_key(ruleOpts.data_type, "OS Version");
                return is_unique_map(ruleOpts.per_run_data) && is_unique_map(os_version);
            },
            good: "Same OS across runs.",
            bad: "Different OS and/or version across runs."
        },
        {
            name: "Total CPUs",
            func: function (ruleOpts: RuleOpts) {
                return is_unique_map(ruleOpts.per_run_data);
            },
            good: "Total CPUs are the same across runs.",
            bad: "Total CPUs are not the same across runs.",
        },
        {
            name: "Kernel Version",
            func: function (ruleOpts: RuleOpts) {
                let versions = map_to_array(ruleOpts.per_run_data);
                for (let i = 0; i < versions.length; i++) {
                    if (versions[i].split(".").length > 2) {
                        versions[i] = versions[i].split(".").slice(0, 2).join(".");
                    } else if (versions[i].split("-").length > 1) {
                        versions[i] = versions[i].split("-")[0];
                    }
                }
                return is_unique_array(versions);
            },
            good: "Kernel versions (major, minor) are the same across all runs.",
            bad: "Kernel versions (major, minor) are not the same across all runs.",
        },
    ],
}

function getSystemInfo(run, container_id, run_data) {
    var data = JSON.parse(run_data);
    data.forEach(function (value, index, arr) {
        var div = document.createElement('div');
        div.id = `${run}-${value.name}-container`;
        addElemToNode(container_id, div);
        var b = document.createElement('b');
        b.style.display = "inline-block";
        b.innerHTML = `${value.name}: `;
        addElemToNode(div.id, b);
        var text_value = document.createElement('div')
        text_value.style.display = "inline-block";
        text_value.innerHTML = ` ${value.value}`;
        addElemToNode(div.id, text_value);
        var p = document.createElement('p');
        addElemToNode(div.id, p);
    })
}

function analytics() {
    let all_analytics = [];
    for (var i = 0; i < all_rules.length; i++) {
        let rules_group = all_rules[i];
        let data_type = rules_group.data_type;
        let analytics = {
            name: rules_group.pretty_name,
            analysis: [],
        }
        for (var j = 0; j < rules_group.rules.length; j++) {
            let rule = rules_group.rules[j];
            let func = rule.func;
            let key = rule.name;
            let good = rule.good;
            let bad = rule.bad;
            let ruleOpts: RuleOpts = {
                data_type: data_type,
                runs: runs_raw,
                key: key,
                all_data: raw_analytics,
                per_run_data: get_data_key(data_type, key),
            }
            let findings = func(ruleOpts);
            if (Array.isArray(findings)) {
                analytics.analysis = analytics.analysis.concat(findings);
            } else {
                let text = findings ? good : bad;
                let status = findings ? Status.Good : Status.NotGood;
                findings = new Finding(text, status);
                analytics.analysis.push(findings);
            }
        }
        all_analytics.push(analytics);
    }
    var table = document.createElement('table');
    table.style.border = 'none';
    table.id = 'analytics-table';
    addElemToNode("system_info", table);
    for (let j = 0; j < all_analytics.length; j++) {
        let analytics = all_analytics[j];
        for (let k = 0; k < analytics.analysis.length; k++) {
            const row = table.insertRow();
            if (k == 0) {
                const key = row.insertCell();
                key.textContent = `${analytics.name}`;
            } else {
                const key = row.insertCell();
                key.textContent = '';
            }
            const tick = row.insertCell();
            tick.textContent = `${analytics.analysis[k].status}`;
            const data = row.insertCell();
            data.textContent = `${analytics.analysis[k].text}`;
        }
    }
}

function systemInfo() {
    if (got_system_info_data) {
        return;
    }
    clear_and_create('systeminfo');
    analytics();
    for (let i = 0; i < system_info_raw_data['runs'].length; i++) {
        let run_name = system_info_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-systeminfo-per-data`;
        let this_run_data = system_info_raw_data['runs'][i];
        setTimeout(() => {
            getSystemInfo(run_name, elem_id, this_run_data['key_values']['values']);
        }, 0);
    }
    got_system_info_data = true;
}
