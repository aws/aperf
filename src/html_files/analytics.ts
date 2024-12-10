class Rule {
    name: string;
    func: Function;
}

class Rules {
    data_type: string;
    pretty_name: string;
    single_run_rules: Array<Rule>;
    all_run_rules: Array<Rule>;
    per_run_rules: Array<Rule>;
}

class RuleOpts {
    data_type: string;
    runs: Array<string>;
    key: string;
    all_data: any;
    base_run: string;
    base_run_data: any;
    other_run_data: Map<string, any>;
    per_run_data: Array<any>;
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
    status: Status;
    recommendation: string;

    constructor(text: string = '', status: Status = Status.NotGood, recommendation: string = '') {
        this.text = text;
        this.status = status;
        this.recommendation = recommendation;
    }

    is_good() {
        this.status = Status.Good;
    }

    is_not_good() {
        this.status = Status.NotGood;
    }
}

function is_unique(values_array) {
    return new Set(values_array).size == 1;
}

let all_rules: Rules[] = [
    system_info_rules,
    cpu_utilization_rules,
];