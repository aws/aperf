let got_system_info_data = false;

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

function systemInfo() {
    if (got_system_info_data) {
        return;
    }
    clear_and_create('systeminfo');
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
