class DataType {
	name: string;
	trueId: string;
	hideClass: string;
	callback;
}

var DataTypes: Map<string, DataType> = new Map<string, DataType>();
DataTypes.set('kernel_config', {name: 'kernel', hideClass: 'kernelDiff', trueId: 'kernel_diff_yes', callback: kernelConfig});
DataTypes.set('sysctl', {name: 'sysctl', hideClass: 'sysctlDiff', trueId: 'sysctl_diff_yes', callback: sysctl});
DataTypes.set('vmstat', {name: 'vmstat', hideClass: 'vmstatHide', trueId: 'vmstat_hide_yes', callback: vmStat});
DataTypes.set('disk_stats', {name: 'diskstat', hideClass: 'diskstatHide', trueId: 'diskstat_hide_yes', callback: diskStats});
DataTypes.set('meminfo', {name: 'meminfo', hideClass: 'meminfoHide', trueId: 'meminfo_hide_yes', callback: meminfo});
DataTypes.set('netstat', {name: 'netstat', hideClass: 'netstatHide', trueId: 'netstat_hide_yes', callback: netStat});
DataTypes.set('interrupts', {name: 'interrupts', hideClass: '', trueId: '', callback: interrupts});
DataTypes.set('cpu_utilization', {name: 'cpuutilization', hideClass: '', trueId: '', callback: cpuUtilization});
DataTypes.set('system_info', {name: 'systeminfo', hideClass: '', trueId: '', callback: systemInfo});
DataTypes.set('flamegraphs', {name: 'flamegraphs', hideClass: '', trueId: '', callback: flamegraphs});
DataTypes.set('top_functions', {name: 'topfunctions', hideClass: '', trueId: '', callback: topFunctions});
DataTypes.set('processes', {name: 'processes', hideClass: '', trueId: '', callback: processes});
DataTypes.set('perfstat', {name: 'perfstat', hideClass: '', trueId: '', callback: perfStat});
DataTypes.set('aperfstat', {name: 'aperfstat', hideClass: '', trueId: '', callback: aperfStat});
DataTypes.set('configure', {name: 'configure', hideClass: '', trueId: '', callback: configure});

function openData(evt: Event, elem: HTMLButtonElement) {
	var tabName: string = elem.name;
	var tabcontent = document.getElementsByClassName('tabcontent');
	var tablinks = document.getElementsByClassName('tablinks');
	for (var i = 0; i < tabcontent.length; i++) {
		(tabcontent[i] as HTMLElement).style.display = "none";
	}
	for (var i = 0; i < tablinks.length; i++) {
		tablinks[i].className = tablinks[i].className.replace(" active", "");
	}
	document.getElementById(tabName).style.display = "block";
	const currentTarget = evt.currentTarget as HTMLButtonElement;
	currentTarget.className += " active";
	display_tab(tabName);
}

function display_tab(name) {
	let datatype = DataTypes.get(name);
	if (datatype.hideClass != "") {
		let queryInput = `input[name="${datatype.hideClass}"]:checked`;
		let checkedId = document.querySelector(queryInput).id;
		datatype.callback(checkedId == datatype.trueId);
	} else {
		datatype.callback();
	}
}

// Tab button click
var elems = document.getElementsByClassName('tablinks');
for (var i=0; i < elems.length; i++) {
	elems[i].addEventListener("click", function(evt: Event) {
		openData(evt, this)
	}, false);
}

// Set Click listener
DataTypes.forEach((datatype: DataType, key: string) => {
	var elems = document.getElementsByClassName(`${datatype.name}-button`);
	for (var j = 0; j < elems.length; j++) {
		elems[j].addEventListener("click", function (evn: Event) {
			if (this.id == datatype.trueId) {
				datatype.callback(true);
			} else {
				datatype.callback(false);
			}
		})
	}
});

var run_width = 100;
var float_style = "none";

function clear_and_create(datatype) {
	clearElements(`${datatype}-runs`);
	runs_raw.forEach(function (value, index, arr) {
		var datatype_div = document.createElement('div');
		datatype_div.id = `${value}-${datatype}`;
		datatype_div.style.float = float_style;
		datatype_div.style.width = `${run_width}%`;
		addElemToNode(`${datatype}-runs`, datatype_div);
		var per_run_datatype = document.createElement('div');
		per_run_datatype.id = `${value}-${datatype}-per-data`;
		addElemToNode(datatype_div.id, per_run_datatype);
	});
}

function create_runs_header() {
	var data = runs_raw;
	float_style = "none";
	if (data.length > 1) {
		float_style = "left";
	}
	run_width = 100 / data.length;
	data.forEach(function(value, index, arr) {
		var run_div = document.createElement('div');
		run_div.id = value;
		run_div.style.float = float_style;
		run_div.style.width = `${run_width}%`;
		run_div.style.border = "1px solid black";
		run_div.style.background = "lightgray";
		run_div.style.opacity = "0.95";
		addElemToNode('header', run_div);
		var run_node_id = run_div.id;

		var h3_run_name = document.createElement('h3');
		h3_run_name.innerHTML = value;
		h3_run_name.style.textAlign = "center";
		addElemToNode(run_node_id, h3_run_name);
	});
	DataTypes.forEach((datatype, key) => {
		clear_and_create(datatype.name);
	});
}

// Set Runs header
create_runs_header();

formGlobalConfig();

// Show landing page
document.getElementById("default").click();
