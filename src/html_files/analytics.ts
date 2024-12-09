class Rule {
    name: string;
    func: Function;
    good: string;
    bad: string;
}

class Rules {
    data_type: string;
    pretty_name: string;
    rules: Array<Rule>;
}

class RuleOpts {
    data_type: string;
    runs: Array<string>;
    key: string;
    all_data: any;
    per_run_data: Map<string, any>;
}

enum Status {
    Good = '✅',
    NotGood = '❌',
}

class Analytics {
    name: string;
    analysis: Array<Finding>;
}

class Finding {
    text: string;
    status: string;
    recommendation: string;

    constructor(text: string = '', status: Status = Status.NotGood, recommendation: string = '') {
        if (status == Status.Good) {
            this.status = '✅';
        } else {
            this.status = '❌';
        }
        this.text = text;
        this.recommendation = recommendation;
    }

    is_good() {
        this.status == '✅';
    }

    is_not_good() {
        this.status == '❌';
    }
}

function is_unique_map(values_map) {
    return new Set([...values_map.values()]).size == 1;
}

function is_unique_array(values_array) {
    return new Set(values_array).size == 1;
}

function map_to_array(values_map) {
    return [...values_map.values()];
}

let all_rules: Rules[] = [
    system_info_rules,
    cpu_utilization_rules,
];