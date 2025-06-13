let got_top_functions_data = false;

function getTopFunctionsInfo(run, container_id, run_data) {
    if (add_no_data_div(container_id, run_data)) {
        return;
    }
    let data = JSON.parse(run_data['values']);
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
    clear_and_create('topfunctions');
    for (let i = 0; i < perf_profile_raw_data['runs'].length; i++) {
        let run_name = perf_profile_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-topfunctions-per-data`;
        let this_run_data = perf_profile_raw_data['runs'][i];
        setTimeout(() => {
            getTopFunctionsInfo(run_name, elem_id, this_run_data['key_values']);
        }, 0);
    }
    got_top_functions_data = true;
}
