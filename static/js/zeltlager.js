var lightbox;

$(document).on('click', '[data-toggle="lightbox"]', function(event) {
	event.preventDefault();
	lightbox = $(this).ekkoLightbox({
		alwaysShowClose: true,
		onContentLoaded: function() {
			const target = this._$element.attr('href');
			if (this._$modalHeader.find('.download-button').length === 0)
				$('<a download class="download-button" target="_blank"><img src="/static/img/download.svg" alt="download"></a>')
					.insertBefore(this._$modalHeader.find('button.close'));

			this._$modalHeader.find('.download-button').attr('href', target).attr('download', target.substr(target.lastIndexOf('/') + 1));
		},
	});
});

window.addEventListener("load", function() {
	$(document).ready(function() {
		$("#showPassword").change(function() {
			if (this.checked) {
				$('#password').attr('type', 'text');
			} else {
				$('#password').attr('type', 'password');
			}
		});
	});
});
