let got_systeminfo_data = "none";
let systeminfo_rules = {
    data_type: "systeminfo",
    pretty_name: "System Info",
    rules: [
        {
            name: "System Name",
            all_run_rule: function* (ruleOpts: RuleOpts): Generator<Finding, void, any> {
                let os_version = get_data_key(ruleOpts.data_type, "OS Version").values();
                if (is_unique(ruleOpts.per_run_data) && is_unique(os_version)) {
                    yield new Finding("Same OS across runs.", Status.Good);
                } else {
                    yield new Finding("Different OS and/or version across runs.");
                }
            },
        },
        {
            name: "Total CPUs",
            all_run_rule: function* (ruleOpts: RuleOpts) : Generator<Finding, void, any>{
                if (is_unique(ruleOpts.per_run_data)) {
                    yield new Finding("Total CPUs are the same across runs.", Status.Good);
                } else {
                    yield new Finding("Total CPUs are not the same across runs.");
                }
            },
        },
        {
            name: "Kernel Version",
            all_run_rule: function* (ruleOpts: RuleOpts) : Generator<Finding, void, any>{
                let versions = ruleOpts.per_run_data;
                for (let i = 0; i < versions.length; i++) {
                    if (versions[i].split(".").length > 2) {
                        versions[i] = versions[i].split(".").slice(0, 2).join(".");
                    } else if (versions[i].split("-").length > 1) {
                        versions[i] = versions[i].split("-")[0];
                    }
                }
                if (is_unique(versions)) {
                    yield new Finding("Kernel versions (major, minor) are the same across all runs.", Status.Good);
                } else {
                    yield new Finding("Kernel versions (major, minor) are not the same across all runs.");
                }
            },
        }
    ]
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

function formRuleOpts(data_type, rule) {
    let per_run_data = get_data_key(data_type, rule.name);
    let base_run_data = per_run_data.get(runs_raw[0]);
    let other_run_data = new Map(per_run_data);
    other_run_data.delete(runs_raw[0]);
    let ruleOpts: RuleOpts = {
        data_type: data_type,
        runs: runs_raw,
        key: rule.name,
        all_data: raw_analytics,
        this_run: undefined,
        this_run_data: undefined,
        base_run: runs_raw[0],
        base_run_data: base_run_data,
        other_run_data: other_run_data,
        per_run_data: [...per_run_data.values()],
    }
    return ruleOpts;
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
            let opts = formRuleOpts(data_type, rule);
            if (runs_raw.length > 1) {
                for (const [key, value] of opts.other_run_data) {
                    opts.this_run = key;
                    opts.this_run_data = value;
                    let gen = rule.per_run_rule?.(opts);
                    if (gen) {
                        let result = gen.next();
                        while (!result.done) {
                            analytics.analysis = analytics.analysis.concat(result.value);
                            result = gen.next();
                        }
                    }
                }
                let gen = rule.all_run_rule?.(opts);
                if (gen) {
                    let result = gen.next();
                    while (!result.done) {
                        analytics.analysis = analytics.analysis.concat(result.value);
                        result = gen.next();
                    }
                }
            }
            for (var k = 0; k < runs_raw.length; k++) {
                let run = runs_raw[k];
                let per_run_data = get_data_key(data_type, rule.name);
                let run_data = per_run_data.get(run);
                opts.base_run = run;
                opts.base_run_data = run_data;
                let gen = rule.single_run_rule?.(opts);
                if (gen) {
                    let result = gen.next();
                    while (!result.done) {
                        analytics.analysis = analytics.analysis.concat(result.value);
                        result = gen.next();
                    }
                }
            }
        }
        all_analytics.push(analytics);
    }
    var table = document.createElement('table');
    table.style.border = 'none';
    table.id = 'analytics-table';
    addElemToNode("findings-data", table);
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

function sutconfig() {
    for (let i = 0; i < systeminfo_raw_data['runs'].length; i++) {
        let run_name = systeminfo_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-systeminfo-per-data`;
        let this_run_data = systeminfo_raw_data['runs'][i];
        setTimeout(() => {
            getSystemInfo(run_name, elem_id, this_run_data['key_values']['values']);
        }, 0);
    }
}

function systemInfo(set) {
    if (set == got_systeminfo_data) {
        return;
    }
    clear_and_create('systeminfo');
    clearElements("findings-data");
    got_systeminfo_data = set;
    switch (set) {
        case 'findings':
            document.getElementById('landing-text').innerHTML = 'Findings';
            analytics();
            break;
        case 'sutconfig':
            document.getElementById('landing-text').innerHTML = 'System Info';
            sutconfig();
            break;
        default:
            return;
    }
}
