// Although IndexVec is a pretty good implementation of a newtype vec, it has the issue that
// it does not (easily) allow getting a new key without also allocating a value
// The idea behind this module is implementing an alternative that works by having two different storages for 