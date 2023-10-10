class DataType {
	name: string;
	trueId: string;
	hideClass: string;
	callback;
}
var DataTypes: Map<string, DataType> = new Map<string, DataType>();
DataTypes.set('kernel', {name: 'kernel', hideClass: 'kernelDiff', trueId: 'kernel_diff_yes', callback: kernelConfig});
DataTypes.set('sysctl', {name: 'sysctl', hideClass: 'sysctlDiff', trueId: 'sysctl_diff_yes', callback: sysctl});
DataTypes.set('vmstat', {name: 'vmstat', hideClass: 'vmstatHide', trueId: 'vmstat_hide_yes', callback: vmStat});
DataTypes.set('diskstat', {name: 'diskstat', hideClass: 'diskstatHide', trueId: 'diskstat_hide_yes', callback: diskStats});
DataTypes.set('meminfo', {name: 'meminfo', hideClass: 'meminfoHide', trueId: 'meminfo_hide_yes', callback: meminfo});
DataTypes.set('netstat', {name: 'netstat', hideClass: 'netstatHide', trueId: 'netstat_hide_yes', callback: netStat});

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
	if (tabName == "system_info") {
		systemInfo();
	}
	if (tabName == "cpu_utilization") {
		cpuUtilization();
	}
	if (tabName == "flamegraphs") {
		flamegraphs();
	}
	if (tabName == "top_functions") {
		topFunctions();
	}
	if (tabName == "processes") {
		processes();
	}
	if (tabName == "interrupts") {
		interrupts();
	}
	if (tabName == "perfstat") {
		perfStat();
	}
	if (tabName == "kernel_config") {
		callChecked('kernel');
	}
	if (tabName == "sysctl") {
		callChecked('sysctl');
	}
	if (tabName == "meminfo") {
		callChecked('meminfo');
	}
	if (tabName == "vmstat") {
		callChecked('vmstat');
	}
	if (tabName == "disk_stats") {
		callChecked('diskstat');
	}
	if (tabName == "netstat") {
		callChecked('netstat');
	}
}

function callChecked(name) {
	let datatype = DataTypes.get(name);
	let queryInput = `input[name="${datatype.hideClass}"]:checked`;
	let id = document.querySelector(queryInput).id;
	if (id == datatype.trueId) {
		datatype.callback(true);
	} else {
		datatype.callback(false);
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
// Show landing page
document.getElementById("default").click();
