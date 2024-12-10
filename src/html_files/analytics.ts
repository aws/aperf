class Rule {
    name: string;
    /** Use this rule to generate Findings irrespective of the number of runs. */
    single_run_rule?: RuleCall;

    /** Use this rule to generate a single Finding for all runs comparison. */
    all_run_rule?: RuleCall;

    /** Use this rule when comparing the base run with every other run. */
    per_run_rule?: RuleCall;
}

interface RuleCall {
    (opts: RuleOpts): Generator<Finding, void, any>;
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
    base_run: string;
    base_run_data: any;
    this_run: any;
    this_run_data: any;
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