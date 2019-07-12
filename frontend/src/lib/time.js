function fmtDuration(seconds) {
    const d = Math.floor(seconds / 86400 )
    const h = Math.floor((seconds % 86400) / 3600 );
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return d + " " + [
        h > 9 ? h : '0' + h,
        m > 9 ? m : '0' + m,
        s > 9 ? s : '0' + s,
    ].filter(a => a).join(':');
}
export {fmtDuration};