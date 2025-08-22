let got_aperf_runlog_data = false;

function getRunlogs(run, container_id, run_data) {
    if (handleNoData(container_id, run_data)) return;

    var div = document.createElement('div');
    div.id = `aperfrunlog-${run}-container`;
    addElemToNode(container_id, div);

    let data = JSON.parse(run_data['values']);
    data[0].data.forEach(function (value, index, arr) {
        var text_value = document.createElement('pre');
        text_value.style.whiteSpace = "pre-wrap";
        text_value.innerHTML = value;
        addElemToNode(div.id, text_value);
    })
}
function aperfRunlog() {
    if (got_aperf_runlog_data) {
        return;
    }
    clear_and_create('aperfrunlog');
    for (let i = 0; i < aperf_runlog_raw_data['runs'].length; i++) {
        let run_name = aperf_runlog_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-aperfrunlog-per-data`;
        let this_run_data = aperf_runlog_raw_data['runs'][i];
        setTimeout(() => {
            getRunlogs(run_name, elem_id, this_run_data['key_values']);
        })
    }
    got_aperf_runlog_data = true;
}