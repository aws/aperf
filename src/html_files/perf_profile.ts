let got_top_functions_data = false;

function getTopFunctionsInfo(run, container_id, run_data) {
    let data = JSON.parse(run_data);
    var div = document.createElement('div');
    div.id = `${run}-top-functions-container`;
    addElemToNode(container_id, div);
    data[0].data.forEach(function (value, index, arr) {
        var text_value = document.createElement('pre');
        text_value.style.whiteSpace = "pre-wrap";
        text_value.style.wordWrap = "break-word";
        text_value.innerHTML = value;
        addElemToNode(div.id, text_value);
    });
}

function topFunctions() {
    if (got_top_functions_data) {
        return;
    }
    clearElements('top-functions-runs');
    runs_raw.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-top-functions`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('top-functions-runs', run_div);
        var run_node_id = run_div.id;

        // Show data
        var per_run_div = document.createElement('div');
        per_run_div.id = `${value}-top-functions-run`;
        addElemToNode(run_node_id, per_run_div);
        for (let i = 0; i < perf_profile_raw_data['runs'].length; i++) {
            if (perf_profile_raw_data['runs'][i]['name'] == value) {
                this_run_data = perf_profile_raw_data['runs'][i];
                setTimeout(() => {
                    getTopFunctionsInfo(value, per_run_div.id, this_run_data['key_values']['values']);
                }, 0);
            }
        }
    })
    got_top_functions_data = true;
}
